use super::blast_radius::compute_blast_radius;
use super::content_signals::detect_content_signals;
use crate::baseline::diff::{
    BaselineScanReport, BaselineStatus, all_findings_new, diff_summary_against_baseline,
};
use crate::baseline::key::{normalized_relative_path, stable_finding_key};
use crate::baseline::model::Baseline;
use crate::config::model::RepoPilotConfig;
use crate::findings::types::{Evidence, Finding, FindingCategory};
use crate::review::diff::{ChangedFile, DiffTarget, load_changed_files, resolve_git_root};
use crate::review::model::{ReviewFindingStatus, ReviewReport};
use crate::review::paths::normalized_review_path;
use crate::review::signals::{BoundarySignal, composites, detect_boundary_signals, tiered};
use crate::risk::{apply_blast_radius_overlay, apply_review_overlay};
use crate::scan::types::ScanSummary;
use std::path::{Path, PathBuf};

pub fn build_review_report(
    summary: ScanSummary,
    scan_path: &Path,
    base: Option<&str>,
    head: Option<&str>,
    baseline: Option<(&Baseline, PathBuf)>,
    config: &RepoPilotConfig,
) -> Result<ReviewReport, crate::review::diff::GitDiffError> {
    let repo_root = resolve_git_root(scan_path)?;
    let target = DiffTarget::from_refs(base, head);
    let pathspec = pathspec_for_scan_path(scan_path, &repo_root);
    let changed_files = load_changed_files(&repo_root, target, pathspec.as_deref())?;
    let boundary_signals = detect_boundary_signals(&changed_files, &config.security_boundary);
    let (behavioral_signals, algorithmic_signals) = detect_content_signals(
        &repo_root,
        target,
        &changed_files,
        config.behavioral.enabled,
        config.algorithmic.enabled,
    );

    let baseline_report = match baseline {
        Some((baseline, baseline_path)) => {
            diff_summary_against_baseline(summary, baseline, baseline_path)
        }
        None => all_findings_new(summary),
    };

    Ok(classify_findings(
        baseline_report,
        repo_root,
        changed_files,
        boundary_signals,
        behavioral_signals,
        algorithmic_signals,
    ))
}

fn classify_findings(
    baseline_report: BaselineScanReport,
    repo_root: PathBuf,
    changed_files: Vec<ChangedFile>,
    mut boundary_signals: Vec<BoundarySignal>,
    behavioral_signals: Vec<crate::review::signals::behavioral::BehavioralSignal>,
    algorithmic_signals: Vec<crate::review::signals::algorithmic::AlgorithmicSignal>,
) -> ReviewReport {
    let mut summary = baseline_report.summary;
    let blast_radius = compute_blast_radius(&summary, &repo_root, &changed_files);
    composites::enrich_blast_radius(
        &mut boundary_signals,
        summary.artifacts.coupling_graph.as_ref(),
        &repo_root,
    );
    let boundary_missing_test =
        composites::missing_test_for_code_boundary(&boundary_signals, &changed_files);

    let mut tiered_signals = tiered::build_tiered(
        &boundary_signals,
        &behavioral_signals,
        &algorithmic_signals,
        &changed_files,
    );
    tiered::enrich_blast_radius(
        &mut tiered_signals,
        summary.artifacts.coupling_graph.as_ref(),
        &repo_root,
    );

    let mut findings: Vec<ReviewFindingStatus> = summary
        .artifacts
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
    let in_diff = findings
        .iter()
        .map(|status| status.in_diff)
        .collect::<Vec<_>>();
    apply_review_overlay(&mut summary.artifacts.findings, &in_diff);
    apply_blast_radius_overlay(&mut summary.artifacts.findings, &repo_root, &blast_radius);
    sort_findings_with_review_status(&mut summary.artifacts.findings, &mut findings);

    ReviewReport {
        summary,
        repo_root,
        baseline_path: baseline_report.baseline_path,
        changed_files,
        blast_radius,
        boundary_signals,
        boundary_missing_test,
        tiered_signals,
        findings,
    }
}

fn sort_findings_with_review_status(
    findings: &mut Vec<Finding>,
    statuses: &mut Vec<ReviewFindingStatus>,
) {
    let mut paired = findings
        .drain(..)
        .zip(statuses.drain(..))
        .collect::<Vec<_>>();
    paired.sort_by(|(left, _), (right, _)| crate::risk::compare_findings(left, right));
    for (finding, status) in paired {
        findings.push(finding);
        statuses.push(status);
    }
}

fn finding_is_in_diff(finding: &Finding, repo_root: &Path, changed_files: &[ChangedFile]) -> bool {
    finding.evidence.iter().any(|evidence| {
        let evidence_path = normalized_relative_path(&evidence.path, repo_root);
        changed_files.iter().any(|changed_file| {
            if changed_file.path_string() != evidence_path {
                return false;
            }

            changed_file.contains_line(evidence.line_start)
                || is_file_level_architecture_evidence(finding, evidence)
        })
    })
}

fn is_file_level_architecture_evidence(finding: &Finding, evidence: &Evidence) -> bool {
    finding.category == FindingCategory::Architecture
        && evidence.line_start == 1
        && evidence.line_end.is_none()
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
