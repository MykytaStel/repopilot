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
    FileRoleEntry, FindingsEntry, ScanCache, config_fingerprint, file_hash_entry,
    relative_cache_path,
};
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::types::{ScanMode, ScanSummary, ScanTimings};
use std::collections::{HashMap, HashSet};
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
    let changed_files =
        load_changed_files(&repo_root, target, pathspec.as_deref()).map_err(diff_error_to_io)?;

    let file_scan_start = Instant::now();
    let file_audits = build_file_audits(config);
    let mut cache = ScanCache::load(&repo_root);
    let fingerprint = config_fingerprint(config);
    let mut facts = ScanFacts {
        root_path: path.to_path_buf(),
        files_discovered: changed_files.len(),
        ..ScanFacts::default()
    };
    let mut languages: HashMap<String, usize> = HashMap::new();
    let mut findings = Vec::new();
    let mut directories = HashSet::new();

    for changed_file in &changed_files {
        if changed_file.status == ChangeStatus::Deleted {
            continue;
        }

        let absolute_path = repo_root.join(&changed_file.path);
        if !absolute_path.is_file() {
            continue;
        }

        if let Some(parent) = changed_file.path.parent()
            && !parent.as_os_str().is_empty()
        {
            directories.insert(parent.to_path_buf());
        }

        let hash_entry = file_hash_entry(&repo_root, &absolute_path)?;
        let cache_path = hash_entry.path.clone();
        cache
            .file_hashes
            .insert(cache_path.clone(), hash_entry.clone());

        if let (Some(role_entry), Some(findings_entry)) = (
            cache.file_roles.get(&cache_path),
            cache.findings.get(&cache_path),
        ) && role_entry.hash == hash_entry.hash
            && findings_entry.hash == hash_entry.hash
            && findings_entry.config_fingerprint == fingerprint
        {
            record_cached_file(&mut facts, &mut languages, role_entry);
            findings.extend(findings_entry.findings.clone());
            continue;
        }

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
                facts.skipped_files_count += 1;
                facts.skipped_bytes = facts.skipped_bytes.saturating_add(per_file.skipped_bytes);
            }
            SkipReason::Binary => {
                facts.binary_files_skipped += 1;
                facts.skipped_bytes = facts.skipped_bytes.saturating_add(per_file.skipped_bytes);
            }
            SkipReason::LowSignal => {
                facts.files_skipped_low_signal += 1;
            }
        }
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

    cache.write(&repo_root)?;

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
    })
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
