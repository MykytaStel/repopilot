use super::super::changed_cache::{
    CacheDecision, cache_decision, normalize_per_file_paths, record_cached_file,
};
use super::super::changed_telemetry::{change_status_label, record_skipped_cache_file};
use super::super::file::{SkipReason, process_file_with_content};
use super::super::summary::build_language_summary;
use super::{ChangedDiscoveryStage, ChangedFileAnalysisStage, ChangedScanEngine};
use crate::analysis::{FileContextFacts, ParsedArtifact, RoleEvidenceFact};
use crate::audits::pipeline::{FileAuditRegistration, registered_file_audits};
use crate::findings::types::Finding;
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::scan::cache::{
    FileRoleEntry, FileRoleEvidenceEntry, FindingsEntry, ScanCache, config_fingerprint,
    file_hash_entry,
};
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::parsed_cache::{ParsedFactsCache, ParsedFactsEntry, content_hash};
use crate::scan::types::{ChangedFileCacheTelemetry, ScanCacheTelemetry};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

impl<'a> ChangedScanEngine<'a> {
    pub(super) fn run_file_analysis(
        &self,
        discovery: &ChangedDiscoveryStage,
    ) -> io::Result<ChangedFileAnalysisStage> {
        let start = Instant::now();
        let parse_nanos_before = crate::analysis::parse::parse_nanos_total();
        let file_audits = registered_file_audits(self.config);
        let cache_load_start = Instant::now();
        let mut cache = ScanCache::load(&discovery.repo_root);
        let mut parsed_cache = ParsedFactsCache::load(&discovery.repo_root);
        let mut cache_telemetry = ScanCacheTelemetry {
            timings: crate::scan::types::ScanCacheTimings {
                load_us: cache_load_start.elapsed().as_micros() as u64,
                ..Default::default()
            },
            ..Default::default()
        };

        let fingerprint = config_fingerprint(self.config);
        let mut facts = ScanFacts {
            root_path: self.path.to_path_buf(),
            files_discovered: discovery.changed_files.len(),
            ..ScanFacts::default()
        };
        let mut languages: HashMap<String, usize> = HashMap::new();
        let mut findings = Vec::new();
        let mut graph_patch_files = Vec::new();
        let mut directories = HashSet::new();
        let mut changed_file_reasons: BTreeMap<String, usize> = BTreeMap::new();

        for changed_file in &discovery.changed_files {
            self.process_changed_file(
                changed_file,
                &discovery.repo_root,
                &fingerprint,
                &file_audits,
                &mut facts,
                &mut languages,
                &mut findings,
                &mut directories,
                &mut cache,
                &mut parsed_cache,
                &mut cache_telemetry,
                &mut changed_file_reasons,
                &mut graph_patch_files,
            )?;
        }

        facts.directories_count = directories.len();
        facts.languages = build_language_summary(languages);

        let parse_us =
            crate::analysis::parse::parse_nanos_total().saturating_sub(parse_nanos_before) / 1_000;

        Ok(ChangedFileAnalysisStage {
            facts,
            findings,
            graph_patch_files,
            cache,
            parsed_cache,
            cache_telemetry,
            changed_file_reasons,
            elapsed_us: start.elapsed().as_micros() as u64,
            parse_us,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn process_changed_file(
        &self,
        changed_file: &ChangedFile,
        repo_root: &Path,
        fingerprint: &str,
        file_audits: &[FileAuditRegistration],
        facts: &mut ScanFacts,
        languages: &mut HashMap<String, usize>,
        findings: &mut Vec<Finding>,
        directories: &mut HashSet<PathBuf>,
        cache: &mut ScanCache,
        parsed_cache: &mut ParsedFactsCache,
        cache_telemetry: &mut ScanCacheTelemetry,
        changed_file_reasons: &mut BTreeMap<String, usize>,
        graph_patch_files: &mut Vec<FileFacts>,
    ) -> io::Result<()> {
        let change_reason = change_status_label(changed_file.status).to_string();
        *changed_file_reasons
            .entry(change_reason.clone())
            .or_insert(0) += 1;

        if changed_file.status == ChangeStatus::Deleted {
            remove_changed_cache_entries(cache, parsed_cache, &changed_file.path);
            record_skipped_cache_file(
                cache_telemetry,
                &changed_file.path,
                &change_reason,
                "deleted",
            );
            return Ok(());
        }

        let absolute_path = repo_root.join(&changed_file.path);
        if !absolute_path.is_file() {
            let reason = if absolute_path.exists() {
                "not-regular-file"
            } else {
                "missing-file"
            };
            remove_changed_cache_entries(cache, parsed_cache, &changed_file.path);
            record_skipped_cache_file(cache_telemetry, &changed_file.path, &change_reason, reason);
            return Ok(());
        }

        if let Some(parent) = changed_file.path.parent()
            && !parent.as_os_str().is_empty()
        {
            directories.insert(parent.to_path_buf());
        }

        let hash_start = Instant::now();
        let hash_entry = file_hash_entry(repo_root, &absolute_path)?;
        cache_telemetry.timings.file_hash_us = cache_telemetry
            .timings
            .file_hash_us
            .saturating_add(hash_start.elapsed().as_micros() as u64);

        let cache_path = hash_entry.path.clone();
        cache
            .file_hashes
            .insert(cache_path.clone(), hash_entry.clone());

        let lookup_start = Instant::now();
        let decision = cache_decision(
            cache.file_roles.get(&cache_path),
            cache.findings.get(&cache_path),
            &hash_entry.hash,
            fingerprint,
        );
        cache_telemetry.timings.lookup_us = cache_telemetry
            .timings
            .lookup_us
            .saturating_add(lookup_start.elapsed().as_micros() as u64);

        let mut cache_reason = match decision {
            CacheDecision::Hit {
                role_entry,
                findings: cached_findings,
            } => {
                self.reuse_cached_file(
                    changed_file,
                    &change_reason,
                    facts,
                    languages,
                    findings,
                    cache_telemetry,
                    graph_patch_files,
                    parsed_cache,
                    *role_entry,
                    cached_findings,
                );
                return Ok(());
            }
            CacheDecision::Miss { reason } => reason,
        };

        let miss_start = Instant::now();
        let roots = crate::scan::workspace::package_roots(repo_root);
        let mut per_file =
            process_file_with_content(&absolute_path, file_audits, self.config, &roots)?;
        normalize_per_file_paths(
            &mut per_file.file_facts.path,
            &mut per_file.findings,
            repo_root,
        );
        if let Some(artifact) = &mut per_file.artifact {
            artifact.rebase_path(per_file.file_facts.path.clone());
        }

        match per_file.skip_reason {
            SkipReason::None => {
                facts.files_analyzed += 1;
                facts.non_empty_lines += per_file.file_facts.non_empty_lines;
                if let Some(language) = &per_file.language {
                    *languages.entry(language.clone()).or_insert(0) += 1;
                }

                if let Some(artifact) = per_file.artifact.as_ref() {
                    let context = &artifact.context;
                    cache.file_roles.insert(
                        cache_path.clone(),
                        FileRoleEntry {
                            path: cache_path.clone(),
                            hash: hash_entry.hash.clone(),
                            language: per_file.language.clone(),
                            non_empty_lines: per_file.file_facts.non_empty_lines,
                            branch_count: per_file.file_facts.branch_count,
                            imports: per_file.file_facts.imports.clone(),
                            deferred_imports: per_file.file_facts.deferred_imports.clone(),
                            roles: context.roles.clone(),
                            role_evidence: context
                                .role_evidence
                                .iter()
                                .map(|evidence| FileRoleEvidenceEntry {
                                    role: evidence.role.clone(),
                                    source: evidence.source.clone(),
                                    reason: evidence.reason.clone(),
                                })
                                .collect(),
                            frameworks: context.frameworks.clone(),
                            runtimes: context.runtimes.clone(),
                            paradigms: context.paradigms.clone(),
                            is_test: context.is_test,
                            has_inline_tests: per_file.file_facts.has_inline_tests,
                            in_executable_package: per_file.file_facts.in_executable_package,
                        },
                    );
                    if let Some(content) = per_file.file_facts.content.as_deref() {
                        parsed_cache.insert(ParsedFactsEntry::from_artifact(
                            content_hash(content),
                            &per_file.file_facts,
                            artifact,
                        ));
                    }
                }

                cache.findings.insert(
                    cache_path,
                    FindingsEntry {
                        path: hash_entry.path,
                        hash: hash_entry.hash,
                        config_fingerprint: fingerprint.to_string(),
                        findings: per_file.findings.clone(),
                    },
                );

                let mut graph_file = per_file.file_facts.clone();
                graph_file.content = None;
                graph_patch_files.push(graph_file);

                if let Some(artifact) = per_file.artifact {
                    facts.insert_artifact(artifact);
                }
                facts.files.push(per_file.file_facts);
                findings.extend(per_file.findings);
            }
            SkipReason::LargeFile => {
                cache_reason = "large-file";
                facts.large_files_skipped += 1;
                facts.skipped_bytes = facts.skipped_bytes.saturating_add(per_file.skipped_bytes);
            }
            SkipReason::Binary => {
                cache_reason = "binary-file";
                facts.binary_files_skipped += 1;
                facts.skipped_bytes = facts.skipped_bytes.saturating_add(per_file.skipped_bytes);
            }
            SkipReason::LowSignal => {
                cache_reason = "low-signal-file";
                facts.files_skipped_low_signal += 1;
            }
        }

        cache_telemetry.timings.miss_scan_us = cache_telemetry
            .timings
            .miss_scan_us
            .saturating_add(miss_start.elapsed().as_micros() as u64);
        cache_telemetry.misses += 1;
        cache_telemetry
            .changed_files
            .push(ChangedFileCacheTelemetry {
                path: changed_file.path.clone(),
                change_reason,
                cache_status: "miss".to_string(),
                cache_reason: cache_reason.to_string(),
            });

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn reuse_cached_file(
        &self,
        changed_file: &ChangedFile,
        change_reason: &str,
        facts: &mut ScanFacts,
        languages: &mut HashMap<String, usize>,
        findings: &mut Vec<Finding>,
        cache_telemetry: &mut ScanCacheTelemetry,
        graph_patch_files: &mut Vec<FileFacts>,
        parsed_cache: &mut ParsedFactsCache,
        role_entry: FileRoleEntry,
        cached_findings: Vec<Finding>,
    ) {
        let reuse_start = Instant::now();
        let context = FileContextFacts {
            roles: role_entry.roles.clone(),
            role_evidence: role_entry
                .role_evidence
                .iter()
                .map(|evidence| RoleEvidenceFact {
                    role: evidence.role.clone(),
                    source: evidence.source.clone(),
                    reason: evidence.reason.clone(),
                })
                .collect(),
            frameworks: role_entry.frameworks.clone(),
            runtimes: role_entry.runtimes.clone(),
            paradigms: role_entry.paradigms.clone(),
            is_test: role_entry.is_test,
        };
        let artifact = parsed_cache
            .lookup(&role_entry.hash, role_entry.language.as_deref())
            .map(|entry| {
                ParsedArtifact::from_parsed_cache_v2(
                    PathBuf::from(&role_entry.path),
                    role_entry.language.clone(),
                    entry.imports.clone(),
                    entry.deferred_imports.clone(),
                    entry.exports.clone(),
                    context.clone(),
                    (&entry.syntax).into(),
                )
            })
            .unwrap_or_else(|| {
                ParsedArtifact::from_legacy_cache(
                    PathBuf::from(&role_entry.path),
                    role_entry.language.clone(),
                    role_entry.imports.clone(),
                    role_entry.deferred_imports.clone(),
                    context,
                )
            });
        let mut graph_file = record_cached_file(facts, languages, &role_entry);
        facts.insert_artifact(artifact);
        graph_file.content = None;
        graph_patch_files.push(graph_file);
        findings.extend(cached_findings);
        cache_telemetry.timings.hit_reuse_us = cache_telemetry
            .timings
            .hit_reuse_us
            .saturating_add(reuse_start.elapsed().as_micros() as u64);
        cache_telemetry.hits += 1;
        cache_telemetry
            .changed_files
            .push(ChangedFileCacheTelemetry {
                path: changed_file.path.clone(),
                change_reason: change_reason.to_string(),
                cache_status: "hit".to_string(),
                cache_reason: "unchanged-content-and-config".to_string(),
            });
    }
}

fn remove_changed_cache_entries(
    cache: &mut ScanCache,
    parsed_cache: &mut ParsedFactsCache,
    path: &Path,
) {
    let cache_path = path.to_string_lossy().replace('\\', "/");
    if let Some(hash_entry) = cache.file_hashes.remove(&cache_path) {
        parsed_cache.remove_content_hash(&hash_entry.hash);
    }
    cache.file_roles.remove(&cache_path);
    cache.findings.remove(&cache_path);
}
