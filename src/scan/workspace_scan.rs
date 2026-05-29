use crate::findings::filter::recompute_summary_metrics;
use crate::findings::types::Finding;
use crate::risk::{apply_cluster_overlay, apply_workspace_hotspot_overlay, sort_findings};
use crate::scan::config::ScanConfig;
use crate::scan::scanner::scan_path_with_config;
use crate::scan::types::{LanguageSummary, ScanDiagnostic, ScanSummary};
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
    let plan = WorkspaceScanPlan::detect(path, scan_config);
    if plan.packages.is_empty() {
        let mut summary = scan_path_with_config(path, scan_config)?;
        summary.artifacts.diagnostics.push(
            ScanDiagnostic::warning(
                "workspace.no-packages",
                "--workspace was requested but no workspace packages were found; scanned as a single package",
            )
            .with_path(path),
        );
        return Ok(summary);
    }

    plan.execute()
}

struct WorkspaceScanPlan<'a> {
    root: &'a Path,
    scan_config: &'a ScanConfig,
    packages: Vec<WorkspacePackage>,
}

struct PackageScanResult {
    name: String,
    result: io::Result<ScanSummary>,
}

impl<'a> WorkspaceScanPlan<'a> {
    fn detect(root: &'a Path, scan_config: &'a ScanConfig) -> Self {
        Self {
            root,
            scan_config,
            packages: detect_workspace_packages(root),
        }
    }

    fn execute(&self) -> io::Result<ScanSummary> {
        let wall_start = Instant::now();
        let mut merged = self.scan_root()?;

        for package in self.scan_packages() {
            match package.result {
                Ok(pkg_summary) => merge_package_summary(&mut merged, pkg_summary, &package.name),
                Err(err) => merged.artifacts.diagnostics.push(ScanDiagnostic::warning(
                    "workspace.package-scan-failed",
                    format!("failed to scan workspace package '{}': {err}", package.name),
                )),
            }
        }

        finalize_workspace_summary(&mut merged, wall_start);
        Ok(merged)
    }

    fn scan_root(&self) -> io::Result<ScanSummary> {
        let root_scan_config = workspace_root_config(self.scan_config, self.root, &self.packages);
        scan_path_with_config(self.root, &root_scan_config)
    }

    fn scan_packages(&self) -> Vec<PackageScanResult> {
        self.packages
            .par_iter()
            .map(|pkg| PackageScanResult {
                name: pkg.name.clone(),
                result: scan_path_with_config(&pkg.root, self.scan_config),
            })
            .collect()
    }
}

fn finalize_workspace_summary(merged: &mut ScanSummary, wall_start: Instant) {
    deduplicate_workspace_findings(&mut merged.artifacts.findings);
    apply_workspace_hotspot_overlay(&mut merged.artifacts.findings);
    apply_cluster_overlay(&mut merged.artifacts.findings);
    sort_findings(&mut merged.artifacts.findings);
    recompute_summary_metrics(merged);
    merged.scan_duration_us = wall_start.elapsed().as_micros() as u64;
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
    for finding in &mut package.artifacts.findings {
        finding.workspace_package = Some(package_name.to_string());
    }

    merged.metrics.files_analyzed += package.metrics.files_analyzed;
    merged.metrics.files_discovered += package.metrics.files_discovered;
    merged.metrics.directories_count += package.metrics.directories_count;
    merged.metrics.non_empty_lines += package.metrics.non_empty_lines;
    merged.metrics.large_files_skipped += package.metrics.large_files_skipped;
    merged.metrics.files_skipped_low_signal += package.metrics.files_skipped_low_signal;
    merged.metrics.binary_files_skipped += package.metrics.binary_files_skipped;
    merged.metrics.files_skipped_by_limit += package.metrics.files_skipped_by_limit;
    merged.metrics.files_skipped_repopilotignore += package.metrics.files_skipped_repopilotignore;

    if merged.repopilotignore_path.is_none() {
        merged.repopilotignore_path = package.repopilotignore_path.clone();
    }
    merged.metrics.skipped_bytes = merged
        .metrics
        .skipped_bytes
        .saturating_add(package.metrics.skipped_bytes);
    merged.metrics.hidden_suggestions_count = merged
        .metrics
        .hidden_suggestions_count
        .saturating_add(package.metrics.hidden_suggestions_count);
    merged
        .artifacts
        .diagnostics
        .extend(package.artifacts.diagnostics);
    merge_language_summaries(&mut merged.metrics.languages, package.metrics.languages);
    merged.artifacts.findings.extend(package.artifacts.findings);
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
        .map(|language| (language.name, language.files_analyzed))
        .collect();

    for language in source {
        *counts.entry(language.name).or_insert(0) += language.files_analyzed;
    }

    let mut merged: Vec<_> = counts
        .into_iter()
        .map(|(name, files_analyzed)| LanguageSummary {
            name,
            files_analyzed,
        })
        .collect();
    merged.sort_by(|left, right| {
        right
            .files_analyzed
            .cmp(&left.files_analyzed)
            .then_with(|| left.name.cmp(&right.name))
    });

    *target = merged;
}
