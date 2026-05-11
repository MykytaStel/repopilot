pub mod diff;
pub mod model;
pub mod render;

use crate::baseline::diff::{
    BaselineScanReport, BaselineStatus, FindingBaselineStatus, all_findings_new,
    diff_summary_against_baseline,
};
use crate::baseline::key::{normalized_relative_path, stable_finding_key};
use crate::baseline::model::Baseline;
use crate::findings::types::Finding;
use crate::review::diff::{ChangedFile, DiffTarget, load_changed_files, resolve_git_root};
use crate::review::model::{ReviewFindingStatus, ReviewReport};
use crate::scan::types::ScanSummary;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub fn build_review_report(
    summary: ScanSummary,
    scan_path: &Path,
    base: Option<&str>,
    head: Option<&str>,
    baseline: Option<(&Baseline, PathBuf)>,
) -> Result<ReviewReport, diff::GitDiffError> {
    let repo_root = resolve_git_root(scan_path)?;
    let target = DiffTarget::from_refs(base, head);
    let pathspec = pathspec_for_scan_path(scan_path, &repo_root);
    let changed_files = load_changed_files(&repo_root, target, pathspec.as_deref())?;

    let baseline_report = match baseline {
        Some((baseline, baseline_path)) => {
            diff_summary_against_baseline(summary, baseline, baseline_path)
        }
        None => all_findings_new(summary),
    };

    Ok(classify_findings(baseline_report, repo_root, changed_files))
}

fn classify_findings(
    baseline_report: BaselineScanReport,
    repo_root: PathBuf,
    changed_files: Vec<ChangedFile>,
) -> ReviewReport {
    let summary = baseline_report.summary;
    let blast_radius = compute_blast_radius(&summary, &repo_root, &changed_files);
    let findings = summary
        .findings
        .iter()
        .enumerate()
        .map(|(index, finding)| ReviewFindingStatus {
            key: stable_finding_key(finding, &summary.root_path),
            in_diff: finding_is_in_diff(finding, &repo_root, &changed_files),
            baseline_status: Some(
                baseline_report
                    .findings
                    .get(index)
                    .map(|finding| finding.status)
                    .unwrap_or(BaselineStatus::New),
            ),
        })
        .collect();

    ReviewReport {
        summary,
        repo_root,
        baseline_path: baseline_report.baseline_path,
        changed_files,
        blast_radius,
        findings,
    }
}

pub fn compute_blast_radius(
    summary: &ScanSummary,
    repo_root: &Path,
    changed_files: &[ChangedFile],
) -> Vec<PathBuf> {
    let graph = match &summary.coupling_graph {
        Some(graph) => graph,
        None => return Vec::new(),
    };

    let changed_paths: BTreeSet<PathBuf> = changed_files
        .iter()
        .map(|file| normalized_review_path(&file.path, repo_root))
        .collect();

    let mut importers_by_target: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();

    for (source, targets) in &graph.edges {
        let source = normalized_review_path(source, repo_root);
        for target in targets {
            importers_by_target
                .entry(normalized_review_path(target, repo_root))
                .or_default()
                .insert(source.clone());
        }
    }

    let mut impacted = BTreeSet::new();

    for changed in &changed_paths {
        if let Some(importers) = importers_by_target.get(changed) {
            for importer in importers {
                if !changed_paths.contains(importer) {
                    impacted.insert(importer.clone());
                }
            }
        }
    }

    impacted.into_iter().collect()
}

pub fn review_report_for_ci(report: &ReviewReport) -> BaselineScanReport {
    let findings = report
        .summary
        .findings
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            let status = report.findings.get(index)?;
            status.in_diff.then_some(FindingBaselineStatus {
                key: status.key.clone(),
                status: status.baseline_status.unwrap_or(BaselineStatus::New),
            })
        })
        .collect();

    let in_diff_findings: Vec<_> = report.in_diff_findings().into_iter().cloned().collect();
    let health_score =
        ScanSummary::compute_health_score(&in_diff_findings, report.summary.lines_of_code);
    BaselineScanReport {
        summary: ScanSummary {
            root_path: report.summary.root_path.clone(),
            files_discovered: report.summary.files_discovered,
            files_count: report.summary.files_count,
            directories_count: report.summary.directories_count,
            lines_of_code: report.summary.lines_of_code,
            skipped_files_count: report.summary.skipped_files_count,
            files_skipped_low_signal: report.summary.files_skipped_low_signal,
            binary_files_skipped: report.summary.binary_files_skipped,
            skipped_bytes: report.summary.skipped_bytes,
            languages: report.summary.languages.clone(),
            detected_frameworks: report.summary.detected_frameworks.clone(),
            framework_projects: report.summary.framework_projects.clone(),
            react_native: report.summary.react_native.clone(),
            findings: in_diff_findings,
            coupling_graph: report.summary.coupling_graph.clone(),
            scan_duration_us: report.summary.scan_duration_us,
            health_score,
            files_skipped_by_limit: report.summary.files_skipped_by_limit,
            files_skipped_repopilotignore: report.summary.files_skipped_repopilotignore,
            repopilotignore_path: report.summary.repopilotignore_path.clone(),
        },
        baseline_path: report.baseline_path.clone(),
        findings,
    }
}

fn finding_is_in_diff(finding: &Finding, repo_root: &Path, changed_files: &[ChangedFile]) -> bool {
    finding.evidence.iter().any(|evidence| {
        let evidence_path = normalized_relative_path(&evidence.path, repo_root);
        changed_files.iter().any(|changed_file| {
            changed_file.path_string() == evidence_path
                && changed_file.contains_line(evidence.line_start)
        })
    })
}

fn pathspec_for_scan_path(scan_path: &Path, repo_root: &Path) -> Option<String> {
    let relative = normalized_review_path(scan_path, repo_root)
        .to_string_lossy()
        .to_string();

    if relative == "." || relative.is_empty() {
        None
    } else {
        Some(relative)
    }
}

fn normalized_review_path(path: &Path, repo_root: &Path) -> PathBuf {
    let repo_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());

    let path = if path.is_absolute() {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        let repo_path = repo_root.join(path);
        repo_path.canonicalize().unwrap_or(repo_path)
    };

    PathBuf::from(normalized_relative_path(&path, &repo_root))
}
