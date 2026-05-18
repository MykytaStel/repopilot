use super::file::{SkipReason, process_file_with_content};
use super::summary::build_language_summary;
use crate::audits::pipeline::build_file_audits;
use crate::baseline::key::stable_finding_key;
use crate::findings::types::Finding;
use crate::review::diff::{
    ChangeStatus, DiffTarget, GitDiffError, load_changed_files, resolve_git_root,
};
use crate::risk::{apply_cluster_overlay, assess_findings};
use crate::scan::cache::{
    CACHE_DIR, FileRoleEntry, FindingsEntry, ScanCache, config_fingerprint, file_hash_entry,
    relative_cache_path,
};
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::types::{
    ChangedFileCacheTelemetry, ChangedFileReasonSummary, ScanCacheTelemetry, ScanMode, ScanSummary,
    ScanTimings,
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
    let repo_root = resolve_git_root(path).map_err(diff_error_to_io)?;
    let pathspec = pathspec_for_scan_path(path, &repo_root);
    let target = match base_ref {
        Some(base) => DiffTarget::Refs { base, head: "HEAD" },
        None => DiffTarget::WorkingTree,
    };
    let changed_files = load_changed_files(&repo_root, target, pathspec.as_deref())
        .map_err(diff_error_to_io)?
        .into_iter()
        .filter(|changed_file| !is_cache_path(&changed_file.path))
        .collect::<Vec<_>>();

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
                facts.files_count += 1;
                facts.lines_of_code += per_file.file_facts.lines_of_code;
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
                            lines_of_code: per_file.file_facts.lines_of_code,
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
                facts.skipped_files_count += 1;
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

    for finding in &mut findings {
        finding.populate_recommendation();
        finding.id = stable_finding_key(finding, &repo_root);
    }
    assess_findings(&mut findings, &facts);
    apply_cluster_overlay(&mut findings);
    super::summary::sort_findings(&mut findings);

    let cache_write_start = Instant::now();
    cache.write(&repo_root)?;
    cache_telemetry.timings.write_us = cache_write_start.elapsed().as_micros() as u64;
    finalize_cache_telemetry(&mut cache_telemetry, changed_file_reasons);

    let scan_duration_us = start.elapsed().as_micros() as u64;
    let health_score = ScanSummary::compute_health_score(&findings, facts.lines_of_code);
    let visible_findings_count = findings.len();

    Ok(ScanSummary {
        root_path: path.to_path_buf(),
        mode: ScanMode::Changed,
        base_ref: base_ref.map(str::to_string),
        changed_files_count: changed_files.len(),
        repo_level_rules_included: false,
        files_discovered: facts.files_discovered,
        files_count: facts.files_count,
        directories_count: facts.directories_count,
        lines_of_code: facts.lines_of_code,
        skipped_files_count: facts.skipped_files_count,
        files_skipped_low_signal: facts.files_skipped_low_signal,
        binary_files_skipped: facts.binary_files_skipped,
        skipped_bytes: facts.skipped_bytes,
        languages: facts.languages,
        detected_frameworks: Vec::new(),
        framework_projects: Vec::new(),
        react_native: None,
        findings,
        coupling_graph: None,
        scan_duration_us,
        health_score,
        visible_findings_count,
        hidden_suggestions_count: 0,
        hidden_suggestions: Vec::new(),
        visibility_profile: None,
        files_skipped_by_limit: 0,
        files_skipped_repopilotignore: 0,
        repopilotignore_path: None,
        scan_timings: Some(ScanTimings {
            file_scan_us,
            framework_detection_us: 0,
            post_scan_audits_us: 0,
        }),
        cache_telemetry: Some(cache_telemetry),
    })
}

enum CacheDecision {
    Hit {
        role_entry: FileRoleEntry,
        findings: Vec<Finding>,
    },
    Miss {
        reason: &'static str,
    },
}

fn cache_decision(
    role_entry: Option<&FileRoleEntry>,
    findings_entry: Option<&FindingsEntry>,
    hash: &str,
    fingerprint: &str,
) -> CacheDecision {
    match (role_entry, findings_entry) {
        (Some(role_entry), Some(findings_entry))
            if role_entry.hash == hash
                && findings_entry.hash == hash
                && findings_entry.config_fingerprint == fingerprint =>
        {
            CacheDecision::Hit {
                role_entry: role_entry.clone(),
                findings: findings_entry.findings.clone(),
            }
        }
        (None, None) => CacheDecision::Miss {
            reason: "missing-cache-entry",
        },
        (None, Some(_)) => CacheDecision::Miss {
            reason: "missing-file-role-cache",
        },
        (Some(_), None) => CacheDecision::Miss {
            reason: "missing-findings-cache",
        },
        (Some(role_entry), Some(findings_entry))
            if role_entry.hash != hash || findings_entry.hash != hash =>
        {
            CacheDecision::Miss {
                reason: "content-changed",
            }
        }
        (Some(_), Some(findings_entry)) if findings_entry.config_fingerprint != fingerprint => {
            CacheDecision::Miss {
                reason: "config-changed",
            }
        }
        (Some(_), Some(_)) => CacheDecision::Miss {
            reason: "cache-mismatch",
        },
    }
}

fn record_skipped_cache_file(
    telemetry: &mut ScanCacheTelemetry,
    path: &Path,
    change_reason: &str,
    cache_reason: &str,
) {
    telemetry.skipped += 1;
    telemetry.changed_files.push(ChangedFileCacheTelemetry {
        path: path.to_path_buf(),
        change_reason: change_reason.to_string(),
        cache_status: "skipped".to_string(),
        cache_reason: cache_reason.to_string(),
    });
}

fn finalize_cache_telemetry(
    telemetry: &mut ScanCacheTelemetry,
    changed_file_reasons: BTreeMap<String, usize>,
) {
    let cached_total = telemetry.hits + telemetry.misses;
    telemetry.hit_rate_percent = if cached_total == 0 {
        0
    } else {
        ((telemetry.hits * 100) / cached_total) as u8
    };
    telemetry.changed_file_reasons = changed_file_reasons
        .into_iter()
        .map(|(reason, count)| ChangedFileReasonSummary { reason, count })
        .collect();

    telemetry.timings.estimated_time_saved_us = if telemetry.hits > 0 && telemetry.misses > 0 {
        let average_miss_us = telemetry.timings.miss_scan_us / telemetry.misses as u64;
        let average_hit_reuse_us = telemetry.timings.hit_reuse_us / telemetry.hits as u64;
        Some(
            average_miss_us
                .saturating_sub(average_hit_reuse_us)
                .saturating_mul(telemetry.hits as u64),
        )
    } else {
        None
    };
}

fn change_status_label(status: ChangeStatus) -> &'static str {
    match status {
        ChangeStatus::Added => "added",
        ChangeStatus::Modified => "modified",
        ChangeStatus::Deleted => "deleted",
        ChangeStatus::Renamed => "renamed",
        ChangeStatus::Untracked => "untracked",
    }
}

fn is_cache_path(path: &Path) -> bool {
    let path = path.to_string_lossy().replace('\\', "/");
    path == CACHE_DIR
        || path
            .strip_prefix(CACHE_DIR)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn record_cached_file(
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
    entry: &FileRoleEntry,
) {
    facts.files_count += 1;
    facts.lines_of_code += entry.lines_of_code;
    if let Some(language) = &entry.language {
        *languages.entry(language.clone()).or_insert(0) += 1;
    }
    facts.files.push(FileFacts {
        path: PathBuf::from(&entry.path),
        language: entry.language.clone(),
        lines_of_code: entry.lines_of_code,
        branch_count: 0,
        imports: Vec::new(),
        content: None,
        has_inline_tests: entry.is_test,
    });
}

fn normalize_per_file_paths(path: &mut PathBuf, findings: &mut [Finding], repo_root: &Path) {
    *path = PathBuf::from(relative_cache_path(repo_root, path));
    for finding in findings {
        for evidence in &mut finding.evidence {
            evidence.path = PathBuf::from(relative_cache_path(repo_root, &evidence.path));
        }
    }
}

fn pathspec_for_scan_path(scan_path: &Path, repo_root: &Path) -> Option<String> {
    let absolute = if scan_path.is_absolute() {
        scan_path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(scan_path)
    };
    let absolute = absolute.canonicalize().unwrap_or(absolute);
    let relative = absolute.strip_prefix(repo_root).ok()?;
    if relative.as_os_str().is_empty() {
        None
    } else {
        Some(relative.to_string_lossy().replace('\\', "/"))
    }
}

fn diff_error_to_io(error: GitDiffError) -> io::Error {
    io::Error::other(error)
}
