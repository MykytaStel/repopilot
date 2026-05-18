use crate::baseline::diff::{BaselineScanReport, BaselineStatus};
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::risk::RiskSummary;
use crate::scan::types::{HiddenSuggestionSummary, LanguageSummary, ScanMode, ScanSummary};
use serde::Serialize;

pub const SCAN_REPORT_SCHEMA_VERSION: &str = "0.10";
pub const REPOPILOT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn render(summary: &ScanSummary) -> Result<String, serde_json::Error> {
    let output = JsonScanReport {
        schema_version: SCAN_REPORT_SCHEMA_VERSION,
        repopilot_version: REPOPILOT_VERSION,
        risk_summary: RiskSummary::from_findings(&summary.findings),
        summary,
    };

    serde_json::to_string_pretty(&output)
}

#[derive(Serialize)]
struct JsonScanReport<'a> {
    schema_version: &'static str,
    repopilot_version: &'static str,
    risk_summary: RiskSummary,
    #[serde(flatten)]
    summary: &'a ScanSummary,
}

pub fn render_with_baseline(
    report: &BaselineScanReport,
    ci_gate: Option<&CiGateResult>,
) -> Result<String, serde_json::Error> {
    let findings = report
        .summary
        .findings
        .iter()
        .enumerate()
        .map(|(index, finding)| FindingWithBaselineStatus {
            finding,
            baseline_status: report.finding_status(index),
        })
        .collect::<Vec<_>>();

    let output = BaselineJsonReport {
        schema_version: SCAN_REPORT_SCHEMA_VERSION,
        repopilot_version: REPOPILOT_VERSION,
        root_path: report.summary.root_path.to_string_lossy().to_string(),
        files_count: report.summary.files_count,
        directories_count: report.summary.directories_count,
        lines_of_code: report.summary.lines_of_code,
        skipped_files_count: report.summary.skipped_files_count,
        skipped_bytes: report.summary.skipped_bytes,
        mode: report.summary.mode,
        base_ref: report.summary.base_ref.as_deref(),
        changed_files_count: report.summary.changed_files_count,
        repo_level_rules_included: report.summary.repo_level_rules_included,
        visible_findings_count: report.summary.findings.len(),
        hidden_suggestions_count: report.summary.hidden_suggestions_count,
        hidden_suggestions: &report.summary.hidden_suggestions,
        visibility_profile: report.summary.visibility_profile.as_deref(),
        languages: &report.summary.languages,
        risk_summary: RiskSummary::from_findings(&report.summary.findings),
        baseline: BaselineJsonMetadata {
            path: report
                .baseline_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            new_findings: report.new_count(),
            existing_findings: report.existing_count(),
        },
        ci_gate: ci_gate.map(CiGateJsonMetadata::from),
        findings,
    };

    serde_json::to_string_pretty(&output)
}

#[derive(Serialize)]
struct BaselineJsonReport<'a> {
    schema_version: &'static str,
    repopilot_version: &'static str,
    root_path: String,
    files_count: usize,
    directories_count: usize,
    lines_of_code: usize,
    skipped_files_count: usize,
    skipped_bytes: u64,
    mode: ScanMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_ref: Option<&'a str>,
    changed_files_count: usize,
    repo_level_rules_included: bool,
    visible_findings_count: usize,
    hidden_suggestions_count: usize,
    #[serde(skip_serializing_if = "hidden_suggestions_empty")]
    hidden_suggestions: &'a Vec<HiddenSuggestionSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visibility_profile: Option<&'a str>,
    languages: &'a [LanguageSummary],
    risk_summary: RiskSummary,
    baseline: BaselineJsonMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    ci_gate: Option<CiGateJsonMetadata>,
    findings: Vec<FindingWithBaselineStatus<'a>>,
}

fn hidden_suggestions_empty(value: &&Vec<HiddenSuggestionSummary>) -> bool {
    value.is_empty()
}

#[derive(Serialize)]
struct BaselineJsonMetadata {
    path: Option<String>,
    new_findings: usize,
    existing_findings: usize,
}

#[derive(Serialize)]
struct CiGateJsonMetadata {
    fail_on: String,
    status: &'static str,
    failed_findings: usize,
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

#[derive(Serialize)]
struct FindingWithBaselineStatus<'a> {
    #[serde(flatten)]
    finding: &'a Finding,
    baseline_status: BaselineStatus,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::diff::{BaselineScanReport, FindingBaselineStatus};
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn json_scan_report_includes_schema_and_tool_versions() {
        let summary = ScanSummary {
            hidden_suggestions: Vec::new(),
            root_path: PathBuf::from("."),
            files_count: 1,
            ..ScanSummary::default()
        };

        let rendered = render(&summary).expect("json render should succeed");
        let value: Value = serde_json::from_str(&rendered).expect("json should parse");

        assert_eq!(value["schema_version"], SCAN_REPORT_SCHEMA_VERSION);
        assert_eq!(value["repopilot_version"], REPOPILOT_VERSION);
        assert_eq!(value["files_count"], 1);
        assert!(value.get("findings").is_some());
    }

    #[test]
    fn baseline_json_report_includes_schema_and_tool_versions() {
        let summary = ScanSummary {
            hidden_suggestions_count: 5,
            hidden_suggestions: vec![HiddenSuggestionSummary {
                intent: "testing-gap".to_string(),
                rule_id: "testing.source-without-test".to_string(),
                category: "testing".to_string(),
                reason: "testing gaps are hidden in the default profile".to_string(),
                count: 5,
            }],
            root_path: PathBuf::from("."),
            files_count: 2,
            ..ScanSummary::default()
        };
        let report = BaselineScanReport {
            summary,
            baseline_path: Some(PathBuf::from(".repopilot/baseline.json")),
            findings: Vec::<FindingBaselineStatus>::new(),
        };

        let rendered =
            render_with_baseline(&report, None).expect("baseline json render should succeed");
        let value: Value = serde_json::from_str(&rendered).expect("json should parse");

        assert_eq!(value["schema_version"], SCAN_REPORT_SCHEMA_VERSION);
        assert_eq!(value["repopilot_version"], REPOPILOT_VERSION);
        assert_eq!(value["files_count"], 2);
        assert_eq!(value["hidden_suggestions_count"], 5);
        assert_eq!(
            value["hidden_suggestions"][0]["rule_id"],
            "testing.source-without-test"
        );
        assert!(value.get("baseline").is_some());
    }
}
