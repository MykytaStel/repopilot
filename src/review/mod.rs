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
        findings,
    }
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

    BaselineScanReport {
        summary: ScanSummary {
            root_path: report.summary.root_path.clone(),
            files_count: report.summary.files_count,
            directories_count: report.summary.directories_count,
            lines_of_code: report.summary.lines_of_code,
            skipped_files_count: report.summary.skipped_files_count,
            skipped_bytes: report.summary.skipped_bytes,
            languages: report.summary.languages.clone(),
            findings: report.in_diff_findings().into_iter().cloned().collect(),
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
    let relative = normalized_relative_path(scan_path, repo_root);

    if relative == "." || relative.is_empty() {
        None
    } else {
        Some(relative)
    }
}
