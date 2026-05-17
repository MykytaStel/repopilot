use crate::findings::types::Finding;
use crate::risk::{apply_cluster_overlay, apply_workspace_hotspot_overlay, sort_findings};
use crate::scan::config::ScanConfig;
use crate::scan::scanner::scan_path_with_config;
use crate::scan::types::{LanguageSummary, ScanSummary};
use crate::scan::workspace::{WorkspacePackage, detect_workspace_packages};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;
use std::time::Instant;

pub fn scan_workspace_with_config(
    path: &Path,
    scan_config: &ScanConfig,
) -> io::Result<ScanSummary> {
    let packages = detect_workspace_packages(path);
    if packages.is_empty() {
        eprintln!(
            "Warning: --workspace specified but no workspace packages found under {}. \
             Falling back to single-package scan.",
            path.display()
        );
        return scan_path_with_config(path, scan_config);
    }

    let wall_start = Instant::now();

    let root_scan_config = workspace_root_config(scan_config, path, &packages);
    let mut merged = scan_path_with_config(path, &root_scan_config)?;

    let pkg_results: Vec<(String, Result<_, _>)> = packages
        .par_iter()
        .map(|pkg| {
            (
                pkg.name.clone(),
                scan_path_with_config(&pkg.root, scan_config),
            )
        })
        .collect();

    for (name, result) in pkg_results {
        match result {
            Ok(pkg_summary) => merge_package_summary(&mut merged, pkg_summary, &name),
            Err(err) => eprintln!("Warning: failed to scan workspace package '{name}': {err}"),
        }
    }

    deduplicate_workspace_findings(&mut merged.findings);
    apply_workspace_hotspot_overlay(&mut merged.findings);
    apply_cluster_overlay(&mut merged.findings);
    sort_findings(&mut merged.findings);
    merged.health_score = ScanSummary::compute_health_score(&merged.findings, merged.lines_of_code);
    merged.scan_duration_us = wall_start.elapsed().as_micros() as u64;
    Ok(merged)
}

fn workspace_root_config(
    scan_config: &ScanConfig,
    root: &Path,
    packages: &[WorkspacePackage],
) -> ScanConfig {
    let mut config = scan_config.clone();
    for package in packages {
        if let Some(relative_path) = workspace_relative_path(root, &package.root) {
            config.ignored_paths.push(relative_path);
        }
    }
    config
}

fn workspace_relative_path(root: &Path, package_root: &Path) -> Option<String> {
    package_root
        .strip_prefix(root)
        .ok()
        .and_then(|path| path.to_str())
        .filter(|path| !path.is_empty())
        .map(|path| path.replace('\\', "/"))
}

fn merge_package_summary(merged: &mut ScanSummary, mut package: ScanSummary, package_name: &str) {
    for finding in &mut package.findings {
        finding.workspace_package = Some(package_name.to_string());
    }

    merged.files_count += package.files_count;
    merged.files_discovered += package.files_discovered;
    merged.directories_count += package.directories_count;
    merged.lines_of_code += package.lines_of_code;
    merged.skipped_files_count += package.skipped_files_count;
    merged.files_skipped_low_signal += package.files_skipped_low_signal;
    merged.binary_files_skipped += package.binary_files_skipped;
    merged.files_skipped_by_limit += package.files_skipped_by_limit;
    merged.files_skipped_repopilotignore += package.files_skipped_repopilotignore;

    if merged.repopilotignore_path.is_none() {
        merged.repopilotignore_path = package.repopilotignore_path.clone();
    }
    merged.skipped_bytes = merged.skipped_bytes.saturating_add(package.skipped_bytes);
    merge_language_summaries(&mut merged.languages, package.languages);
    merged.findings.extend(package.findings);
}

fn deduplicate_workspace_findings(findings: &mut Vec<Finding>) {
    let mut seen: HashSet<(String, std::path::PathBuf, usize)> = HashSet::new();
    findings.retain(|f| {
        let key = f
            .evidence
            .first()
            .map(|e| (f.rule_id.clone(), e.path.clone(), e.line_start))
            .unwrap_or_else(|| (f.rule_id.clone(), std::path::PathBuf::new(), 0));
        seen.insert(key)
    });
}

fn merge_language_summaries(target: &mut Vec<LanguageSummary>, source: Vec<LanguageSummary>) {
    let mut counts: HashMap<String, usize> = target
        .drain(..)
        .map(|language| (language.name, language.files_count))
        .collect();

    for language in source {
        *counts.entry(language.name).or_insert(0) += language.files_count;
    }

    let mut merged: Vec<_> = counts
        .into_iter()
        .map(|(name, files_count)| LanguageSummary { name, files_count })
        .collect();
    merged.sort_by(|left, right| {
        right
            .files_count
            .cmp(&left.files_count)
            .then_with(|| left.name.cmp(&right.name))
    });

    *target = merged;
}
