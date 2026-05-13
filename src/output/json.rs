use crate::baseline::diff::{BaselineScanReport, BaselineStatus};
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::scan::types::LanguageSummary;
use crate::scan::types::ScanSummary;
use serde::Serialize;

pub const SCAN_REPORT_SCHEMA_VERSION: &str = "0.9";
pub const REPOPILOT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn render(summary: &ScanSummary) -> Result<String, serde_json::Error> {
    let output = JsonScanReport {
        schema_version: SCAN_REPORT_SCHEMA_VERSION,
        repopilot_version: REPOPILOT_VERSION,
        summary,
    };

    serde_json::to_string_pretty(&output)
}

#[derive(Serialize)]
struct JsonScanReport<'a> {
    schema_version: &'static str,
    repopilot_version: &'static str,
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
        languages: &report.summary.languages,
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
    languages: &'a [LanguageSummary],
    baseline: BaselineJsonMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    ci_gate: Option<CiGateJsonMetadata>,
    findings: Vec<FindingWithBaselineStatus<'a>>,
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
        assert!(value.get("baseline").is_some());
    }
}
