use crate::baseline::diff::{BaselineScanReport, BaselineStatus};
use crate::baseline::gate::CiGateResult;
use crate::findings::feedback::LocalFeedbackReport;
use crate::findings::types::Finding;
use crate::frameworks::{DetectedFramework, FrameworkProject, ReactNativeArchitectureProfile};
use crate::graph::CouplingGraph;
use crate::report::quality::build_signal_quality_summary;
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

pub const SCAN_REPORT_SCHEMA_VERSION: &str = "0.15";
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
    pub scan_duration_us: u64,
    pub health_score: u8,
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
            changed_files_count: summary.changed_files_count,
            repo_level_rules_included: summary.repo_level_rules_included,
            files_discovered: summary.files_discovered,
            files_analyzed: summary.files_analyzed,
            directories_count: summary.directories_count,
            non_empty_lines: summary.non_empty_lines,
            large_files_skipped: summary.large_files_skipped,
            files_skipped_low_signal: summary.files_skipped_low_signal,
            binary_files_skipped: summary.binary_files_skipped,
            skipped_bytes: summary.skipped_bytes,
            languages: &summary.languages,
            findings: &summary.findings,
            detected_frameworks: &summary.detected_frameworks,
            framework_projects: &summary.framework_projects,
            react_native: summary.react_native.as_ref(),
            coupling_graph: summary.coupling_graph.as_ref(),
            scan_duration_us: summary.scan_duration_us,
            health_score: summary.health_score,
            visible_findings_count: summary.visible_findings_count,
            hidden_suggestions_count: summary.hidden_suggestions_count,
            hidden_suggestions: &summary.hidden_suggestions,
            visibility_profile: summary.visibility_profile.as_deref(),
            files_skipped_by_limit: summary.files_skipped_by_limit,
            files_skipped_repopilotignore: summary.files_skipped_repopilotignore,
            repopilotignore_path: summary.repopilotignore_path.as_ref(),
            scan_timings: summary.scan_timings.as_ref(),
            cache_telemetry: summary.cache_telemetry.as_ref(),
            local_feedback: summary.local_feedback.as_ref(),
            diagnostics: &summary.diagnostics,
            risk_summary: RiskSummary::from_findings(&summary.findings),
            signal_quality: build_signal_quality_summary(&summary.findings),
        }
    }
}

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
    pub visible_findings_count: usize,
    pub hidden_suggestions_count: usize,
    #[serde(skip_serializing_if = "hidden_suggestions_empty")]
    pub hidden_suggestions: &'a [HiddenSuggestionSummary],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility_profile: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_telemetry: Option<&'a ScanCacheTelemetry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_feedback: Option<&'a LocalFeedbackReport>,
    #[serde(skip_serializing_if = "diagnostics_empty")]
    pub diagnostics: &'a [ScanDiagnostic],
    pub languages: &'a [LanguageSummary],
    pub risk_summary: RiskSummary,
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
            visible_findings_count: report.summary.findings.len(),
            hidden_suggestions_count: report.summary.hidden_suggestions_count,
            hidden_suggestions: &report.summary.hidden_suggestions,
            visibility_profile: report.summary.visibility_profile.as_deref(),
            cache_telemetry: report.summary.cache_telemetry.as_ref(),
            local_feedback: report.summary.local_feedback.as_ref(),
            diagnostics: &report.summary.diagnostics,
            languages: &report.summary.languages,
            risk_summary: RiskSummary::from_findings(&report.summary.findings),
            signal_quality: build_signal_quality_summary(&report.summary.findings),
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
    pub review: ReviewJsonMetadata,
    pub risk_summary: RiskSummary,
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
            files_analyzed: report.summary.files_analyzed,
            directories_count: report.summary.directories_count,
            non_empty_lines: report.summary.non_empty_lines,
            changed_files: &report.changed_files,
            blast_radius: report
                .blast_radius
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect(),
            review: ReviewJsonMetadata {
                in_diff_findings: report.in_diff_count(),
                out_of_diff_findings: report.out_of_diff_count(),
                new_in_diff_findings: report.new_in_diff_count(),
                existing_in_diff_findings: report.existing_in_diff_count(),
                severity_counts: report.severity_counts(),
            },
            risk_summary: RiskSummary::from_findings(&report.summary.findings),
            signal_quality: build_signal_quality_summary(&report.summary.findings),
            baseline: ReviewBaselineJsonMetadata {
                path: report
                    .baseline_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
            },
            ci_gate: ci_gate.map(CiGateJsonMetadata::from),
            local_feedback: report.summary.local_feedback.as_ref(),
            diagnostics: &report.summary.diagnostics,
            findings: report
                .summary
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

fn hidden_suggestions_empty(value: &&[HiddenSuggestionSummary]) -> bool {
    value.is_empty()
}

fn diagnostics_empty(value: &&[ScanDiagnostic]) -> bool {
    value.is_empty()
}

#[derive(Debug, Serialize)]
pub struct BaselineJsonMetadata {
    pub path: Option<String>,
    pub new_findings: usize,
    pub existing_findings: usize,
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

#[derive(Debug, Serialize)]
pub struct FindingWithBaselineStatus<'a> {
    #[serde(flatten)]
    pub finding: &'a Finding,
    pub baseline_status: BaselineStatus,
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

    if schema_version == Some(SCAN_REPORT_SCHEMA_VERSION)
        && report_kind == Some("scan")
        && report_schema_version == Some(SCAN_REPORT_SCHEMA_VERSION)
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
