use crate::baseline::diff::{BaselineScanReport, BaselineStatus};
use crate::baseline::gate::CiGateResult;
use crate::findings::feedback::LocalFeedbackReport;
use crate::findings::types::Finding;
use crate::frameworks::{DetectedFramework, FrameworkProject, ReactNativeArchitectureProfile};
use crate::graph::CouplingGraph;
use crate::graph::context::{ContextGraphCacheInfo, ContextGraphSummary};
use crate::review::diff::ChangedFile;
use crate::review::model::{ReviewReport, SeverityCounts};
use crate::risk::RiskSummary;
use crate::scan::types::{
    HiddenSuggestionSummary, LanguageSummary, ScanCacheTelemetry, ScanDiagnostic, ScanMode,
    ScanSummary, ScanTimings,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;
use std::path::PathBuf;

pub const SCAN_REPORT_SCHEMA_VERSION: &str = "0.17";
const ACCEPTED_SCAN_REPORT_SCHEMA_VERSIONS: &[&str] = &["0.15", "0.16", SCAN_REPORT_SCHEMA_VERSION];
pub const REPOPILOT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportEnvelope {
    pub kind: String,
    pub schema_version: String,
    pub repopilot_version: String,
}

impl ReportEnvelope {
    pub fn new(kind: impl Into<String>, schema_version: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            schema_version: schema_version.into(),
            repopilot_version: REPOPILOT_VERSION.to_string(),
        }
    }

    pub fn scan() -> Self {
        Self::new("scan", SCAN_REPORT_SCHEMA_VERSION)
    }

    pub fn baseline_scan() -> Self {
        Self::new("baseline-scan", SCAN_REPORT_SCHEMA_VERSION)
    }

    pub fn review() -> Self {
        Self::new("review", SCAN_REPORT_SCHEMA_VERSION)
    }

    pub fn sarif() -> Self {
        Self::new("sarif", "2.1.0")
    }

    pub fn receipt(schema_version: u32) -> Self {
        Self::new("receipt", schema_version.to_string())
    }
}

#[derive(Debug, Serialize)]
pub struct ScanJsonReport<'a> {
    pub schema_version: &'static str,
    pub repopilot_version: &'static str,
    pub report: ReportEnvelope,
    pub root_path: String,
    pub mode: ScanMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<&'a str>,
    pub changed_files_count: usize,
    pub repo_level_rules_included: bool,
    pub files_discovered: usize,
    pub files_analyzed: usize,
    pub directories_count: usize,
    pub non_empty_lines: usize,
    pub large_files_skipped: usize,
    pub files_skipped_low_signal: usize,
    pub binary_files_skipped: usize,
    pub skipped_bytes: u64,
    pub languages: &'a [LanguageSummary],
    pub findings: &'a [Finding],
    pub detected_frameworks: &'a [DetectedFramework],
    pub framework_projects: &'a [FrameworkProject],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub react_native: Option<&'a ReactNativeArchitectureProfile>,
    pub coupling_graph: Option<&'a CouplingGraph>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_graph_summary: Option<&'a ContextGraphSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_graph_cache: Option<&'a ContextGraphCacheInfo>,
    pub scan_duration_us: u64,
    pub health_score: u8,
    pub raw_findings_count: usize,
    pub visible_findings_count: usize,
    pub hidden_suggestions_count: usize,
    #[serde(skip_serializing_if = "hidden_suggestions_empty")]
    pub hidden_suggestions: &'a [HiddenSuggestionSummary],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility_profile: Option<&'a str>,
    pub files_skipped_by_limit: usize,
    pub files_skipped_repopilotignore: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repopilotignore_path: Option<&'a PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_timings: Option<&'a ScanTimings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_telemetry: Option<&'a ScanCacheTelemetry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_feedback: Option<&'a LocalFeedbackReport>,
    #[serde(skip_serializing_if = "diagnostics_empty")]
    pub diagnostics: &'a [ScanDiagnostic],
    pub risk_summary: RiskSummary,
    pub raw_signal_quality: crate::findings::quality::SignalQualitySummary,
    pub visible_signal_quality: crate::findings::quality::SignalQualitySummary,
    pub signal_quality: crate::findings::quality::SignalQualitySummary,
}

impl<'a> ScanJsonReport<'a> {
    pub fn from_summary(summary: &'a ScanSummary) -> Self {
        Self {
            schema_version: SCAN_REPORT_SCHEMA_VERSION,
            repopilot_version: REPOPILOT_VERSION,
            report: ReportEnvelope::scan(),
            root_path: summary.root_path.to_string_lossy().to_string(),
            mode: summary.mode,
            base_ref: summary.base_ref.as_deref(),
            changed_files_count: summary.metrics.changed_files_count,
            repo_level_rules_included: summary.repo_level_rules_included,
            files_discovered: summary.metrics.files_discovered,
            files_analyzed: summary.metrics.files_analyzed,
            directories_count: summary.metrics.directories_count,
            non_empty_lines: summary.metrics.non_empty_lines,
            large_files_skipped: summary.metrics.large_files_skipped,
            files_skipped_low_signal: summary.metrics.files_skipped_low_signal,
            binary_files_skipped: summary.metrics.binary_files_skipped,
            skipped_bytes: summary.metrics.skipped_bytes,
            languages: &summary.metrics.languages,
            findings: &summary.artifacts.findings,
            detected_frameworks: &summary.artifacts.detected_frameworks,
            framework_projects: &summary.artifacts.framework_projects,
            react_native: summary.artifacts.react_native.as_ref(),
            coupling_graph: summary.artifacts.coupling_graph.as_ref(),
            context_graph_summary: summary.artifacts.context_graph_summary.as_ref(),
            context_graph_cache: summary.artifacts.context_graph_cache.as_ref(),
            scan_duration_us: summary.scan_duration_us,
            health_score: summary.metrics.health_score,
            raw_findings_count: summary.metrics.raw_findings_count,
            visible_findings_count: summary.metrics.visible_findings_count,
            hidden_suggestions_count: summary.metrics.hidden_suggestions_count,
            hidden_suggestions: &summary.artifacts.hidden_suggestions,
            visibility_profile: summary.visibility_profile.as_deref(),
            files_skipped_by_limit: summary.metrics.files_skipped_by_limit,
            files_skipped_repopilotignore: summary.metrics.files_skipped_repopilotignore,
            repopilotignore_path: summary.repopilotignore_path.as_ref(),
            scan_timings: summary.scan_timings.as_ref(),
            cache_telemetry: summary.cache_telemetry.as_ref(),
            local_feedback: summary.local_feedback.as_ref(),
            diagnostics: &summary.artifacts.diagnostics,
            risk_summary: RiskSummary::from_findings(&summary.artifacts.findings),
            raw_signal_quality: summary.artifacts.raw_signal_quality.clone(),
            visible_signal_quality: summary.artifacts.visible_signal_quality.clone(),
            signal_quality: summary.artifacts.signal_quality.clone(),
        }
    }
}

mod baseline;
mod review;

pub use baseline::{BaselineJsonMetadata, BaselineJsonReport, FindingWithBaselineStatus};
pub use review::{
    ReviewBaselineJsonMetadata, ReviewJsonFinding, ReviewJsonMetadata, ReviewJsonReport,
};

pub(super) fn hidden_suggestions_empty(value: &&[HiddenSuggestionSummary]) -> bool {
    value.is_empty()
}

pub(super) fn diagnostics_empty(value: &&[ScanDiagnostic]) -> bool {
    value.is_empty()
}

#[derive(Debug, Serialize)]
pub struct CiGateJsonMetadata {
    pub fail_on: String,
    pub status: &'static str,
    pub failed_findings: usize,
}

impl From<&CiGateResult> for CiGateJsonMetadata {
    fn from(result: &CiGateResult) -> Self {
        Self {
            fail_on: result.label(),
            status: if result.passed() { "passed" } else { "failed" },
            failed_findings: result.failed_findings,
        }
    }
}

pub fn parse_scan_summary_json(content: &str) -> Result<ScanSummary, serde_json::Error> {
    let value: Value = serde_json::from_str(content)?;
    parse_scan_summary_value(value)
}

pub fn parse_scan_summary_value(value: Value) -> Result<ScanSummary, serde_json::Error> {
    validate_current_scan_report(&value)?;
    serde_json::from_value(value)
}

fn validate_current_scan_report(value: &Value) -> Result<(), serde_json::Error> {
    let schema_version = value.get("schema_version").and_then(Value::as_str);
    let report = value.get("report").and_then(Value::as_object);
    let report_kind = report
        .and_then(|report| report.get("kind"))
        .and_then(Value::as_str);
    let report_schema_version = report
        .and_then(|report| report.get("schema_version"))
        .and_then(Value::as_str);

    if schema_version.is_some_and(is_accepted_scan_schema_version)
        && report_kind == Some("scan")
        && report_schema_version.is_some_and(is_accepted_scan_schema_version)
    {
        return Ok(());
    }

    Err(serde_json::Error::io(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "unsupported scan report schema; expected scan report schema {}",
            SCAN_REPORT_SCHEMA_VERSION
        ),
    )))
}

fn is_accepted_scan_schema_version(version: &str) -> bool {
    ACCEPTED_SCAN_REPORT_SCHEMA_VERSIONS.contains(&version)
}
