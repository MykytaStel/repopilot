use super::super::changed_cache::{
    CacheDecision, cache_decision, normalize_per_file_paths, record_cached_file,
};
use super::super::changed_telemetry::{change_status_label, record_skipped_cache_file};
use super::super::file::{SkipReason, process_file_with_content};
use super::super::summary::build_language_summary;
use super::{ChangedDiscoveryStage, ChangedFileAnalysisStage, ChangedScanEngine};
use crate::audits::pipeline::build_file_audits;
use crate::findings::types::Finding;
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::scan::cache::{
    FileRoleEntry, FindingsEntry, ScanCache, config_fingerprint, file_hash_entry,
};
use crate::scan::facts::{FileFacts, ScanFacts};
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
        let file_audits = build_file_audits(self.config);
        let cache_load_start = Instant::now();
        let mut cache = ScanCache::load(&discovery.repo_root);
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
        file_audits: &[Box<dyn crate::audits::traits::FileAudit>],
        facts: &mut ScanFacts,
        languages: &mut HashMap<String, usize>,
        findings: &mut Vec<Finding>,
        directories: &mut HashSet<PathBuf>,
        cache: &mut ScanCache,
        cache_telemetry: &mut ScanCacheTelemetry,
        changed_file_reasons: &mut BTreeMap<String, usize>,
        graph_patch_files: &mut Vec<FileFacts>,
    ) -> io::Result<()> {
        let change_reason = change_status_label(changed_file.status).to_string();
        *changed_file_reasons
            .entry(change_reason.clone())
            .or_insert(0) += 1;

        if changed_file.status == ChangeStatus::Deleted {
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
                    *role_entry,
                    cached_findings,
                );
                return Ok(());
            }
            CacheDecision::Miss { reason } => reason,
        };

        let miss_start = Instant::now();
        let mut per_file = process_file_with_content(&absolute_path, file_audits, self.config)?;
        normalize_per_file_paths(
            &mut per_file.file_facts.path,
            &mut per_file.findings,
            repo_root,
        );

        match per_file.skip_reason {
            SkipReason::None => {
                facts.files_analyzed += 1;
                facts.non_empty_lines += per_file.file_facts.non_empty_lines;
                if let Some(language) = &per_file.language {
                    *languages.entry(language.clone()).or_insert(0) += 1;
                }

                if let Some(context) = per_file.context {
                    cache.file_roles.insert(
                        cache_path.clone(),
                        FileRoleEntry {
                            path: cache_path.clone(),
                            hash: hash_entry.hash.clone(),
                            language: per_file.language.clone(),
                            non_empty_lines: per_file.file_facts.non_empty_lines,
                            imports: per_file.file_facts.imports.clone(),
                            roles: context.roles,
                            frameworks: context.frameworks,
                            runtimes: context.runtimes,
                            paradigms: context.paradigms,
                            is_test: context.is_test,
                        },
                    );
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
        role_entry: FileRoleEntry,
        cached_findings: Vec<Finding>,
    ) {
        let reuse_start = Instant::now();
        let mut graph_file = record_cached_file(facts, languages, &role_entry);
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
