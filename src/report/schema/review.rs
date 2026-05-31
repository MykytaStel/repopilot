use super::*;
use crate::review::signals::BoundarySignal;

#[derive(Debug, Serialize)]
pub struct ReviewJsonReport<'a> {
    pub schema_version: &'static str,
    pub repopilot_version: &'static str,
    pub report: ReportEnvelope,
    pub root_path: String,
    pub git_root: String,
    pub files_analyzed: usize,
    pub directories_count: usize,
    pub non_empty_lines: usize,
    pub changed_files: &'a [ChangedFile],
    pub blast_radius: Vec<String>,
    pub boundary_signals: &'a [BoundarySignal],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_graph_summary: Option<&'a ContextGraphSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_graph_cache: Option<&'a ContextGraphCacheInfo>,
    pub review: ReviewJsonMetadata,
    pub risk_summary: RiskSummary,
    pub raw_signal_quality: crate::findings::quality::SignalQualitySummary,
    pub visible_signal_quality: crate::findings::quality::SignalQualitySummary,
    pub signal_quality: crate::findings::quality::SignalQualitySummary,
    pub baseline: ReviewBaselineJsonMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ci_gate: Option<CiGateJsonMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_feedback: Option<&'a LocalFeedbackReport>,
    #[serde(skip_serializing_if = "diagnostics_empty")]
    pub diagnostics: &'a [ScanDiagnostic],
    pub findings: Vec<ReviewJsonFinding<'a>>,
}

impl<'a> ReviewJsonReport<'a> {
    pub fn from_report(report: &'a ReviewReport, ci_gate: Option<&CiGateResult>) -> Self {
        Self {
            schema_version: SCAN_REPORT_SCHEMA_VERSION,
            repopilot_version: REPOPILOT_VERSION,
            report: ReportEnvelope::review(),
            root_path: report.summary.root_path.to_string_lossy().to_string(),
            git_root: report.repo_root.to_string_lossy().to_string(),
            files_analyzed: report.summary.metrics.files_analyzed,
            directories_count: report.summary.metrics.directories_count,
            non_empty_lines: report.summary.metrics.non_empty_lines,
            changed_files: &report.changed_files,
            blast_radius: report
                .blast_radius
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect(),
            boundary_signals: &report.boundary_signals,
            context_graph_summary: report.summary.artifacts.context_graph_summary.as_ref(),
            context_graph_cache: report.summary.artifacts.context_graph_cache.as_ref(),
            review: ReviewJsonMetadata {
                in_diff_findings: report.in_diff_count(),
                out_of_diff_findings: report.out_of_diff_count(),
                new_in_diff_findings: report.new_in_diff_count(),
                existing_in_diff_findings: report.existing_in_diff_count(),
                boundary_signals: report.boundary_signals.len(),
                severity_counts: report.severity_counts(),
            },
            risk_summary: RiskSummary::from_findings(&report.summary.artifacts.findings),
            raw_signal_quality: report.summary.artifacts.raw_signal_quality.clone(),
            visible_signal_quality: report.summary.artifacts.visible_signal_quality.clone(),
            signal_quality: report.summary.artifacts.signal_quality.clone(),
            baseline: ReviewBaselineJsonMetadata {
                path: report
                    .baseline_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
            },
            ci_gate: ci_gate.map(CiGateJsonMetadata::from),
            local_feedback: report.summary.local_feedback.as_ref(),
            diagnostics: &report.summary.artifacts.diagnostics,
            findings: report
                .summary
                .artifacts
                .findings
                .iter()
                .enumerate()
                .map(|(index, finding)| ReviewJsonFinding {
                    finding,
                    in_diff: report
                        .finding_status(index)
                        .map(|status| status.in_diff)
                        .unwrap_or(false),
                    baseline_status: report
                        .finding_status(index)
                        .and_then(|status| status.baseline_status),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ReviewJsonMetadata {
    pub in_diff_findings: usize,
    pub out_of_diff_findings: usize,
    pub new_in_diff_findings: usize,
    pub existing_in_diff_findings: usize,
    pub boundary_signals: usize,
    pub severity_counts: SeverityCounts,
}

#[derive(Debug, Serialize)]
pub struct ReviewBaselineJsonMetadata {
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReviewJsonFinding<'a> {
    #[serde(flatten)]
    pub finding: &'a Finding,
    pub in_diff: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_status: Option<BaselineStatus>,
}
