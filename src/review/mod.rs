pub mod diff;
pub mod model;
pub(crate) mod paths;
pub mod render;
pub mod signals;

use crate::baseline::diff::{
    BaselineScanReport, BaselineStatus, FindingBaselineStatus, all_findings_new,
    diff_summary_against_baseline,
};
use crate::baseline::key::{normalized_relative_path, stable_finding_key};
use crate::baseline::model::Baseline;
use crate::config::model::RepoPilotConfig;
use crate::findings::filter::recompute_summary_metrics;
use crate::findings::quality::summarize_signal_quality;
use crate::findings::types::{Evidence, Finding, FindingCategory};
use crate::review::diff::{ChangedFile, DiffTarget, load_changed_files, resolve_git_root};
use crate::review::model::{ReviewFindingStatus, ReviewReport};
use crate::review::paths::normalized_review_path;
use crate::review::signals::algorithmic::{self, AlgorithmicSignal};
use crate::review::signals::behavioral::{self, BehavioralSignal};
use crate::review::signals::content;
use crate::review::signals::{BoundarySignal, composites, detect_boundary_signals, tiered};
use crate::risk::{apply_blast_radius_overlay, apply_review_overlay};
use crate::scan::types::{ScanArtifacts, ScanMetadata, ScanMetrics, ScanSummary};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub fn build_review_report(
    summary: ScanSummary,
    scan_path: &Path,
    base: Option<&str>,
    head: Option<&str>,
    baseline: Option<(&Baseline, PathBuf)>,
    config: &RepoPilotConfig,
) -> Result<ReviewReport, diff::GitDiffError> {
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

/// Run the content-based detectors (behavioral + algorithmic) over each changed
/// file, re-reading its pre/post source via the content bridge. Each is gated by
/// its config toggle; both off means no git reads at all.
fn detect_content_signals(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed_files: &[ChangedFile],
    behavioral_enabled: bool,
    algorithmic_enabled: bool,
) -> (Vec<BehavioralSignal>, Vec<AlgorithmicSignal>) {
    let mut behavioral = Vec::new();
    let mut algorithmic = Vec::new();
    if !behavioral_enabled && !algorithmic_enabled {
        return (behavioral, algorithmic);
    }

    for file in changed_files {
        let post = content::post_change_source(repo_root, file, target);
        let pre = content::pre_change_source(repo_root, file, target);

        if behavioral_enabled {
            if let Some(post) = &post {
                behavioral.extend(behavioral::detect_behavioral_added(file, post));
            }
            behavioral.extend(behavioral::detect_behavioral_removed(
                file,
                pre.as_ref(),
                post.as_ref(),
            ));
        }
        if algorithmic_enabled {
            algorithmic.extend(algorithmic::detect_algorithmic(
                file,
                pre.as_ref(),
                post.as_ref(),
            ));
        }
    }

    (behavioral, algorithmic)
}

fn classify_findings(
    baseline_report: BaselineScanReport,
    repo_root: PathBuf,
    changed_files: Vec<ChangedFile>,
    mut boundary_signals: Vec<BoundarySignal>,
    behavioral_signals: Vec<BehavioralSignal>,
    algorithmic_signals: Vec<AlgorithmicSignal>,
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

pub fn compute_blast_radius(
    summary: &ScanSummary,
    repo_root: &Path,
    changed_files: &[ChangedFile],
) -> Vec<PathBuf> {
    let graph = match &summary.artifacts.coupling_graph {
        Some(graph) => graph,
        None => return Vec::new(),
    };

    let changed_paths: BTreeSet<PathBuf> = changed_files
        .iter()
        .map(|file| normalized_review_path(&file.path, repo_root))
        .collect();

    let importers_by_target = composites::build_importers_by_target(graph, repo_root);

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
        .artifacts
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
    let in_diff_findings_count = in_diff_findings.len();
    let in_diff_signal_quality = summarize_signal_quality(&in_diff_findings);
    let mut summary = ScanSummary {
        metadata: ScanMetadata {
            root_path: report.summary.root_path.clone(),
            mode: report.summary.mode,
            base_ref: report.summary.base_ref.clone(),
            repo_level_rules_included: report.summary.repo_level_rules_included,
            scan_duration_us: report.summary.scan_duration_us,
            scan_timings: None,
            cache_telemetry: report.summary.cache_telemetry.clone(),
            local_feedback: report.summary.local_feedback.clone(),
            visibility_profile: report.summary.visibility_profile.clone(),
            repopilotignore_path: report.summary.repopilotignore_path.clone(),
        },
        metrics: ScanMetrics {
            files_discovered: report.summary.metrics.files_discovered,
            files_analyzed: report.summary.metrics.files_analyzed,
            directories_count: report.summary.metrics.directories_count,
            non_empty_lines: report.summary.metrics.non_empty_lines,
            large_files_skipped: report.summary.metrics.large_files_skipped,
            files_skipped_low_signal: report.summary.metrics.files_skipped_low_signal,
            binary_files_skipped: report.summary.metrics.binary_files_skipped,
            skipped_bytes: report.summary.metrics.skipped_bytes,
            files_skipped_by_limit: report.summary.metrics.files_skipped_by_limit,
            files_skipped_repopilotignore: report.summary.metrics.files_skipped_repopilotignore,
            changed_files_count: report.summary.metrics.changed_files_count,
            health_score: 0,
            raw_findings_count: in_diff_findings_count,
            visible_findings_count: 0,
            hidden_suggestions_count: report.summary.metrics.hidden_suggestions_count,
            languages: report.summary.metrics.languages.clone(),
        },
        artifacts: ScanArtifacts {
            findings: in_diff_findings,
            detected_frameworks: report.summary.artifacts.detected_frameworks.clone(),
            framework_projects: report.summary.artifacts.framework_projects.clone(),
            react_native: report.summary.artifacts.react_native.clone(),
            coupling_graph: report.summary.artifacts.coupling_graph.clone(),
            context_graph_summary: report.summary.artifacts.context_graph_summary.clone(),
            context_graph_cache: report.summary.artifacts.context_graph_cache.clone(),
            hidden_suggestions: Vec::new(),
            diagnostics: report.summary.artifacts.diagnostics.clone(),
            raw_signal_quality: in_diff_signal_quality.clone(),
            visible_signal_quality: in_diff_signal_quality.clone(),
            signal_quality: in_diff_signal_quality,
        },
    };
    recompute_summary_metrics(&mut summary);

    BaselineScanReport {
        summary,
        baseline_path: report.baseline_path.clone(),
        findings,
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
