use crate::baseline::diff::{BaselineScanReport, BaselineStatus, FindingBaselineStatus};
use crate::findings::filter::recompute_summary_metrics;
use crate::findings::quality::summarize_signal_quality;
use crate::review::model::ReviewReport;
use crate::scan::types::{ScanArtifacts, ScanMetadata, ScanMetrics, ScanSummary};

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
