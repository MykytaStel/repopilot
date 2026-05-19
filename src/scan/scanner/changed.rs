use super::changed_cache::{
    CacheDecision, cache_decision, normalize_per_file_paths, record_cached_file,
};
use super::changed_git::collect_changed_scope;
use super::changed_telemetry::{
    change_status_label, finalize_cache_telemetry, record_skipped_cache_file,
};
use super::file::{SkipReason, process_file_with_content};
use super::summary::{ScanSummaryParts, build_language_summary, build_scan_summary};
use crate::audits::pipeline::build_file_audits;
use crate::baseline::key::stable_finding_key;
use crate::frameworks::{
    DetectedFramework, detect_framework_projects, detect_frameworks,
    detect_react_native_architecture,
};
use crate::graph::{CouplingGraph, build_coupling_graph};
use crate::review::diff::ChangeStatus;
use crate::risk::{apply_cluster_overlay, apply_graph_overlay, assess_findings};
use crate::scan::cache::{
    FileRoleEntry, FindingsEntry, ScanCache, config_fingerprint, file_hash_entry,
    relative_cache_path,
};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use crate::scan::types::{
    ChangedFileCacheTelemetry, ScanCacheTelemetry, ScanMode, ScanSummary, ScanTimings,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub fn scan_changed_with_config(
    path: &Path,
    config: &ScanConfig,
    base_ref: Option<&str>,
) -> io::Result<ScanSummary> {
    let start = Instant::now();
    let changed_scope = collect_changed_scope(path, base_ref)?;
    let repo_root = changed_scope.repo_root;
    let changed_files = changed_scope.changed_files;

    let file_scan_start = Instant::now();
    let file_audits = build_file_audits(config);
    let cache_load_start = Instant::now();
    let mut cache = ScanCache::load(&repo_root);
    let mut cache_telemetry = ScanCacheTelemetry {
        timings: crate::scan::types::ScanCacheTimings {
            load_us: cache_load_start.elapsed().as_micros() as u64,
            ..Default::default()
        },
        ..Default::default()
    };
    let fingerprint = config_fingerprint(config);
    let mut facts = ScanFacts {
        root_path: path.to_path_buf(),
        files_discovered: changed_files.len(),
        ..ScanFacts::default()
    };
    let mut languages: HashMap<String, usize> = HashMap::new();
    let mut findings = Vec::new();
    let mut directories = HashSet::new();
    let mut changed_file_reasons: BTreeMap<String, usize> = BTreeMap::new();

    for changed_file in &changed_files {
        let change_reason = change_status_label(changed_file.status).to_string();
        *changed_file_reasons
            .entry(change_reason.clone())
            .or_insert(0) += 1;

        if changed_file.status == ChangeStatus::Deleted {
            record_skipped_cache_file(
                &mut cache_telemetry,
                &changed_file.path,
                &change_reason,
                "deleted",
            );
            continue;
        }

        let absolute_path = repo_root.join(&changed_file.path);
        if !absolute_path.is_file() {
            let reason = if absolute_path.exists() {
                "not-regular-file"
            } else {
                "missing-file"
            };
            record_skipped_cache_file(
                &mut cache_telemetry,
                &changed_file.path,
                &change_reason,
                reason,
            );
            continue;
        }

        if let Some(parent) = changed_file.path.parent()
            && !parent.as_os_str().is_empty()
        {
            directories.insert(parent.to_path_buf());
        }

        let hash_start = Instant::now();
        let hash_entry = file_hash_entry(&repo_root, &absolute_path)?;
        cache_telemetry.timings.file_hash_us = cache_telemetry
            .timings
            .file_hash_us
            .saturating_add(hash_start.elapsed().as_micros() as u64);
        let cache_path = hash_entry.path.clone();
        cache
            .file_hashes
            .insert(cache_path.clone(), hash_entry.clone());

        let lookup_start = Instant::now();
        let cache_decision = cache_decision(
            cache.file_roles.get(&cache_path),
            cache.findings.get(&cache_path),
            &hash_entry.hash,
            &fingerprint,
        );
        cache_telemetry.timings.lookup_us = cache_telemetry
            .timings
            .lookup_us
            .saturating_add(lookup_start.elapsed().as_micros() as u64);

        let mut reason = match cache_decision {
            CacheDecision::Hit {
                role_entry,
                findings: cached_findings,
            } => {
                let reuse_start = Instant::now();
                record_cached_file(&mut facts, &mut languages, &role_entry);
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
                        change_reason,
                        cache_status: "hit".to_string(),
                        cache_reason: "unchanged-content-and-config".to_string(),
                    });
                continue;
            }
            CacheDecision::Miss { reason } => reason,
        };

        let miss_start = Instant::now();
        let mut per_file = process_file_with_content(&absolute_path, &file_audits, config)?;
        normalize_per_file_paths(
            &mut per_file.file_facts.path,
            &mut per_file.findings,
            &repo_root,
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
                        config_fingerprint: fingerprint.clone(),
                        findings: per_file.findings.clone(),
                    },
                );

                facts.files.push(per_file.file_facts);
                findings.extend(per_file.findings);
            }
            SkipReason::LargeFile => {
                reason = "large-file";
                facts.large_files_skipped += 1;
                facts.skipped_bytes = facts.skipped_bytes.saturating_add(per_file.skipped_bytes);
            }
            SkipReason::Binary => {
                reason = "binary-file";
                facts.binary_files_skipped += 1;
                facts.skipped_bytes = facts.skipped_bytes.saturating_add(per_file.skipped_bytes);
            }
            SkipReason::LowSignal => {
                reason = "low-signal-file";
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
                cache_reason: reason.to_string(),
            });
    }

    facts.directories_count = directories.len();
    facts.languages = build_language_summary(languages);
    let file_scan_us = file_scan_start.elapsed().as_micros() as u64;

    let framework_start = Instant::now();
    let mut repo_context =
        super::collection::collect_scan_facts_without_content(&repo_root, config)?;
    repo_context.detected_frameworks = detect_frameworks(&repo_root);
    repo_context.framework_projects = detect_framework_projects(&repo_root);
    repo_context.react_native = detect_react_native_profile(&repo_context);
    let coupling_graph =
        relative_coupling_graph(build_coupling_graph(&repo_context, &repo_root), &repo_root);
    facts.detected_frameworks = repo_context.detected_frameworks.clone();
    facts.framework_projects = repo_context.framework_projects.clone();
    facts.react_native = repo_context.react_native.clone();
    let framework_detection_us = framework_start.elapsed().as_micros() as u64;

    for finding in &mut findings {
        finding.populate_recommendation();
        finding.id = stable_finding_key(finding, &repo_root);
    }
    assess_findings(&mut findings, &repo_context);
    apply_graph_overlay(&mut findings, &coupling_graph);
    apply_cluster_overlay(&mut findings);
    super::summary::sort_findings(&mut findings);

    let cache_write_start = Instant::now();
    cache.write(&repo_root)?;
    cache_telemetry.timings.write_us = cache_write_start.elapsed().as_micros() as u64;
    finalize_cache_telemetry(&mut cache_telemetry, changed_file_reasons);

    facts.root_path = path.to_path_buf();
    let scan_duration_us = start.elapsed().as_micros() as u64;

    Ok(build_scan_summary(
        facts,
        findings,
        ScanSummaryParts {
            mode: ScanMode::Changed,
            base_ref: base_ref.map(str::to_string),
            changed_files_count: changed_files.len(),
            repo_level_rules_included: false,
            coupling_graph: None,
            scan_duration_us,
            scan_timings: Some(ScanTimings {
                file_scan_us,
                framework_detection_us,
                post_scan_audits_us: 0,
            }),
            cache_telemetry: Some(cache_telemetry),
            diagnostics: Vec::new(),
        },
    ))
}

fn detect_react_native_profile(
    facts: &ScanFacts,
) -> Option<crate::frameworks::ReactNativeArchitectureProfile> {
    if facts
        .detected_frameworks
        .iter()
        .any(|f| matches!(f, DetectedFramework::ReactNative { .. }))
    {
        let profile = detect_react_native_architecture(&facts.root_path);
        if profile.detected {
            return Some(profile);
        }
    }
    None
}

fn relative_coupling_graph(graph: CouplingGraph, repo_root: &Path) -> CouplingGraph {
    CouplingGraph {
        edges: graph
            .edges
            .into_iter()
            .map(|(source, targets)| {
                (
                    PathBuf::from(relative_cache_path(repo_root, &source)),
                    targets
                        .into_iter()
                        .map(|target| PathBuf::from(relative_cache_path(repo_root, &target)))
                        .collect(),
                )
            })
            .collect(),
        nodes: graph
            .nodes
            .into_iter()
            .map(|node| PathBuf::from(relative_cache_path(repo_root, &node)))
            .collect(),
    }
}
