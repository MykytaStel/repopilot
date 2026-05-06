use crate::baseline::diff::{BaselineScanReport, BaselineStatus};
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::scan::types::LanguageSummary;
use crate::scan::types::ScanSummary;
use serde::Serialize;

pub fn render(summary: &ScanSummary) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(summary)
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
        root_path: report.summary.root_path.to_string_lossy().to_string(),
        files_count: report.summary.files_count,
        directories_count: report.summary.directories_count,
        lines_of_code: report.summary.lines_of_code,
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
    root_path: String,
    files_count: usize,
    directories_count: usize,
    lines_of_code: usize,
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
