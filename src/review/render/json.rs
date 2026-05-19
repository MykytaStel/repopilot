use crate::baseline::diff::BaselineStatus;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::report::schema::{REPOPILOT_VERSION, ReportEnvelope, SCAN_REPORT_SCHEMA_VERSION};
use crate::review::diff::ChangedFile;
use crate::review::model::{ReviewReport, SeverityCounts};
use crate::risk::RiskSummary;
use crate::scan::types::ScanDiagnostic;
use serde::Serialize;

pub fn render_json(
    report: &ReviewReport,
    ci_gate: Option<&CiGateResult>,
) -> Result<String, serde_json::Error> {
    let findings = report
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
        .collect::<Vec<_>>();

    let output = ReviewJsonReport {
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
        baseline: ReviewBaselineJsonMetadata {
            path: report
                .baseline_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
        },
        ci_gate: ci_gate.map(ReviewCiGateJsonMetadata::from),
        diagnostics: &report.summary.diagnostics,
        findings,
    };

    serde_json::to_string_pretty(&output)
}

#[derive(Serialize)]
struct ReviewJsonReport<'a> {
    schema_version: &'static str,
    repopilot_version: &'static str,
    report: ReportEnvelope,
    root_path: String,
    git_root: String,
    files_analyzed: usize,
    directories_count: usize,
    non_empty_lines: usize,
    changed_files: &'a [ChangedFile],
    blast_radius: Vec<String>,
    review: ReviewJsonMetadata,
    risk_summary: RiskSummary,
    baseline: ReviewBaselineJsonMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    ci_gate: Option<ReviewCiGateJsonMetadata>,
    #[serde(skip_serializing_if = "diagnostics_empty")]
    diagnostics: &'a [ScanDiagnostic],
    findings: Vec<ReviewJsonFinding<'a>>,
}

fn diagnostics_empty(value: &&[ScanDiagnostic]) -> bool {
    value.is_empty()
}

#[derive(Serialize)]
struct ReviewJsonMetadata {
    in_diff_findings: usize,
    out_of_diff_findings: usize,
    new_in_diff_findings: usize,
    existing_in_diff_findings: usize,
    severity_counts: SeverityCounts,
}

#[derive(Serialize)]
struct ReviewBaselineJsonMetadata {
    path: Option<String>,
}

#[derive(Serialize)]
struct ReviewCiGateJsonMetadata {
    fail_on: String,
    status: &'static str,
    failed_findings: usize,
}

impl From<&CiGateResult> for ReviewCiGateJsonMetadata {
    fn from(result: &CiGateResult) -> Self {
        Self {
            fail_on: result.label(),
            status: if result.passed() { "passed" } else { "failed" },
            failed_findings: result.failed_findings,
        }
    }
}

#[derive(Serialize)]
struct ReviewJsonFinding<'a> {
    #[serde(flatten)]
    finding: &'a Finding,
    in_diff: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    baseline_status: Option<BaselineStatus>,
}
