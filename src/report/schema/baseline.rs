use super::*;

#[derive(Debug, Serialize)]
pub struct BaselineJsonReport<'a> {
    pub schema_version: &'static str,
    pub repopilot_version: &'static str,
    pub report: ReportEnvelope,
    pub root_path: String,
    pub files_analyzed: usize,
    pub directories_count: usize,
    pub non_empty_lines: usize,
    pub large_files_skipped: usize,
    pub skipped_bytes: u64,
    pub mode: ScanMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<&'a str>,
    pub changed_files_count: usize,
    pub repo_level_rules_included: bool,
    pub raw_findings_count: usize,
    pub visible_findings_count: usize,
    pub hidden_suggestions_count: usize,
    #[serde(skip_serializing_if = "hidden_suggestions_empty")]
    pub hidden_suggestions: &'a [HiddenSuggestionSummary],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility_profile: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_telemetry: Option<&'a ScanCacheTelemetry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_graph_summary: Option<&'a ContextGraphSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_graph_cache: Option<&'a ContextGraphCacheInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_feedback: Option<&'a LocalFeedbackReport>,
    #[serde(skip_serializing_if = "diagnostics_empty")]
    pub diagnostics: &'a [ScanDiagnostic],
    pub languages: &'a [LanguageSummary],
    pub risk_summary: RiskSummary,
    pub raw_signal_quality: crate::findings::quality::SignalQualitySummary,
    pub visible_signal_quality: crate::findings::quality::SignalQualitySummary,
    pub signal_quality: crate::findings::quality::SignalQualitySummary,
    pub baseline: BaselineJsonMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ci_gate: Option<CiGateJsonMetadata>,
    pub findings: Vec<FindingWithBaselineStatus<'a>>,
}

impl<'a> BaselineJsonReport<'a> {
    pub fn from_report(report: &'a BaselineScanReport, ci_gate: Option<&CiGateResult>) -> Self {
        Self {
            schema_version: SCAN_REPORT_SCHEMA_VERSION,
            repopilot_version: REPOPILOT_VERSION,
            report: ReportEnvelope::baseline_scan(),
            root_path: report.summary.root_path.to_string_lossy().to_string(),
            files_analyzed: report.summary.files_analyzed,
            directories_count: report.summary.directories_count,
            non_empty_lines: report.summary.non_empty_lines,
            large_files_skipped: report.summary.large_files_skipped,
            skipped_bytes: report.summary.skipped_bytes,
            mode: report.summary.mode,
            base_ref: report.summary.base_ref.as_deref(),
            changed_files_count: report.summary.changed_files_count,
            repo_level_rules_included: report.summary.repo_level_rules_included,
            raw_findings_count: report.summary.raw_findings_count,
            visible_findings_count: report.summary.visible_findings_count,
            hidden_suggestions_count: report.summary.hidden_suggestions_count,
            hidden_suggestions: &report.summary.hidden_suggestions,
            visibility_profile: report.summary.visibility_profile.as_deref(),
            cache_telemetry: report.summary.cache_telemetry.as_ref(),
            context_graph_summary: report.summary.context_graph_summary.as_ref(),
            context_graph_cache: report.summary.context_graph_cache.as_ref(),
            local_feedback: report.summary.local_feedback.as_ref(),
            diagnostics: &report.summary.diagnostics,
            languages: &report.summary.languages,
            risk_summary: RiskSummary::from_findings(&report.summary.findings),
            raw_signal_quality: report.summary.raw_signal_quality.clone(),
            visible_signal_quality: report.summary.visible_signal_quality.clone(),
            signal_quality: report.summary.signal_quality.clone(),
            baseline: BaselineJsonMetadata {
                path: report
                    .baseline_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                new_findings: report.new_count(),
                existing_findings: report.existing_count(),
            },
            ci_gate: ci_gate.map(CiGateJsonMetadata::from),
            findings: report
                .summary
                .findings
                .iter()
                .enumerate()
                .map(|(index, finding)| FindingWithBaselineStatus {
                    finding,
                    baseline_status: report.finding_status(index),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BaselineJsonMetadata {
    pub path: Option<String>,
    pub new_findings: usize,
    pub existing_findings: usize,
}

#[derive(Debug, Serialize)]
pub struct FindingWithBaselineStatus<'a> {
    #[serde(flatten)]
    pub finding: &'a Finding,
    pub baseline_status: BaselineStatus,
}
