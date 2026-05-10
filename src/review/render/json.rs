use crate::baseline::diff::BaselineStatus;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::review::diff::ChangedFile;
use crate::review::model::{ReviewReport, SeverityCounts};
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
        root_path: report.summary.root_path.to_string_lossy().to_string(),
        git_root: report.repo_root.to_string_lossy().to_string(),
        files_count: report.summary.files_count,
        directories_count: report.summary.directories_count,
        lines_of_code: report.summary.lines_of_code,
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
        baseline: ReviewBaselineJsonMetadata {
            path: report
                .baseline_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
        },
        ci_gate: ci_gate.map(ReviewCiGateJsonMetadata::from),
        findings,
    };

    serde_json::to_string_pretty(&output)
}

#[derive(Serialize)]
struct ReviewJsonReport<'a> {
    root_path: String,
    git_root: String,
    files_count: usize,
    directories_count: usize,
    lines_of_code: usize,
    changed_files: &'a [ChangedFile],
    blast_radius: Vec<String>,
    review: ReviewJsonMetadata,
    baseline: ReviewBaselineJsonMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    ci_gate: Option<ReviewCiGateJsonMetadata>,
    findings: Vec<ReviewJsonFinding<'a>>,
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
