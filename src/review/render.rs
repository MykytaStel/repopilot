use crate::baseline::diff::BaselineStatus;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::output::OutputFormat;
use crate::output::render_helpers::escape_table_cell;
use crate::review::diff::ChangedFile;
use crate::review::model::{ReviewReport, SeverityCounts};
use serde::Serialize;

pub fn render_console(report: &ReviewReport, ci_gate: Option<&CiGateResult>) -> String {
    let mut output = String::new();

    output.push_str("RepoPilot Review\n");
    output.push_str(&format!("Path: {}\n", report.summary.root_path.display()));
    output.push_str(&format!("Git root: {}\n", report.repo_root.display()));
    match &report.baseline_path {
        Some(path) => output.push_str(&format!("Baseline: {}\n", path.display())),
        None => output.push_str("Baseline: none (all findings treated as new)\n"),
    }
    output.push('\n');

    output.push_str(&format!("Changed files: {}\n", report.changed_files.len()));
    output.push_str(&format!("In-diff findings: {}\n", report.in_diff_count()));
    output.push_str(&format!(
        "Out-of-diff findings: {}\n",
        report.out_of_diff_count()
    ));
    output.push_str(&format!(
        "New in-diff findings: {}\n",
        report.new_in_diff_count()
    ));
    output.push_str(&format!(
        "Existing in-diff findings: {}\n",
        report.existing_in_diff_count()
    ));

    if let Some(ci_gate) = ci_gate {
        let status = if ci_gate.passed() { "passed" } else { "failed" };
        output.push_str(&format!("CI gate: {status} ({})\n", ci_gate.label()));
    }

    output.push_str("\nChanged files:\n");
    if report.changed_files.is_empty() {
        output.push_str("  No changed files found\n");
    } else {
        for file in &report.changed_files {
            output.push_str(&format!(
                "  {:?} {}{}\n",
                file.status,
                file.path.display(),
                render_ranges_suffix(file)
            ));
        }
    }

    render_blast_radius(&mut output, report);

    render_findings_group(&mut output, "In-diff findings", &report.in_diff_findings());
    render_findings_group(
        &mut output,
        "Out-of-diff findings",
        &report.out_of_diff_findings(),
    );

    output
}

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

pub fn render(
    report: &ReviewReport,
    format: OutputFormat,
    ci_gate: Option<&CiGateResult>,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(render_console(report, ci_gate)),
        OutputFormat::Json => render_json(report, ci_gate),
        OutputFormat::Markdown => Ok(render_markdown(report, ci_gate)),
        OutputFormat::Html | OutputFormat::Sarif => {
            unreachable!("HTML and SARIF are not supported for the review command")
        }
    }
}

pub fn render_markdown(report: &ReviewReport, ci_gate: Option<&CiGateResult>) -> String {
    let mut output = String::new();

    output.push_str("# RepoPilot Review Report\n\n");
    output.push_str("## Summary\n\n");
    output.push_str(&format!(
        "- **Path:** `{}`\n",
        report.summary.root_path.display()
    ));
    output.push_str(&format!(
        "- **Git root:** `{}`\n",
        report.repo_root.display()
    ));
    output.push_str(&format!(
        "- **Changed files:** {}\n",
        report.changed_files.len()
    ));
    output.push_str(&format!(
        "- **In-diff findings:** {}\n",
        report.in_diff_count()
    ));
    output.push_str(&format!(
        "- **Out-of-diff findings:** {}\n",
        report.out_of_diff_count()
    ));

    if let Some(ci_gate) = ci_gate {
        let status = if ci_gate.passed() { "passed" } else { "failed" };
        output.push_str(&format!(
            "- **CI gate:** {status} (`{}`)\n",
            ci_gate.label()
        ));
    }

    output.push_str("\n## Changed Files\n\n");
    if report.changed_files.is_empty() {
        output.push_str("No changed files found.\n\n");
    } else {
        output.push_str("| Status | Path | Ranges |\n");
        output.push_str("| --- | --- | --- |\n");
        for file in &report.changed_files {
            output.push_str(&format!(
                "| {:?} | `{}` | {} |\n",
                file.status,
                file.path.display(),
                escape_table_cell(&render_ranges(file))
            ));
        }
        output.push('\n');
    }

    render_markdown_blast_radius(&mut output, report);

    render_markdown_findings_group(
        &mut output,
        "In-Diff Findings",
        report
            .in_diff_findings()
            .into_iter()
            .map(|finding| (finding, status_for_finding(report, finding)))
            .collect::<Vec<_>>()
            .as_slice(),
    );
    render_markdown_findings_group(
        &mut output,
        "Out-Of-Diff Findings",
        report
            .out_of_diff_findings()
            .into_iter()
            .map(|finding| (finding, status_for_finding(report, finding)))
            .collect::<Vec<_>>()
            .as_slice(),
    );

    output
}

fn render_blast_radius(output: &mut String, report: &ReviewReport) {
    if report.blast_radius.is_empty() {
        return;
    }

    output.push_str("\nBlast radius:\n");
    output.push_str("  The following files import changed files and may need extra review:\n");

    for path in &report.blast_radius {
        output.push_str(&format!("  - {}\n", path.display()));
    }
}

fn render_markdown_blast_radius(output: &mut String, report: &ReviewReport) {
    if report.blast_radius.is_empty() {
        return;
    }

    output.push_str("## Blast Radius\n\n");
    output.push_str("The following files import changed files and may need extra review:\n\n");

    for path in &report.blast_radius {
        output.push_str(&format!("- `{}`\n", path.display()));
    }

    output.push('\n');
}

fn render_findings_group(output: &mut String, label: &str, findings: &[&Finding]) {
    output.push_str(&format!("\n{label}: {}\n", findings.len()));

    if findings.is_empty() {
        return;
    }

    for finding in findings {
        output.push_str(&format!(
            "  [{}] {} — {}\n",
            finding.severity_label(),
            finding.rule_id,
            finding.title
        ));

        for evidence in &finding.evidence {
            output.push_str(&format!(
                "    Evidence: {}:{} — {}\n",
                evidence.path.display(),
                evidence.line_start,
                evidence.snippet.trim()
            ));
        }
    }
}

fn render_markdown_findings_group(
    output: &mut String,
    label: &str,
    findings: &[(&Finding, Option<BaselineStatus>)],
) {
    output.push_str(&format!("## {label}\n\n"));

    if findings.is_empty() {
        output.push_str("No findings.\n\n");
        return;
    }

    output.push_str("| Severity | Baseline | Rule | Title | Evidence |\n");
    output.push_str("| --- | --- | --- | --- | --- |\n");

    for (finding, status) in findings {
        let evidence = finding
            .evidence
            .first()
            .map(|evidence| {
                format!(
                    "`{}:{}` — {}",
                    evidence.path.display(),
                    evidence.line_start,
                    evidence.snippet.trim()
                )
            })
            .unwrap_or_else(|| "No evidence".to_string());

        output.push_str(&format!(
            "| {} | {} | `{}` | {} | {} |\n",
            finding.severity_label(),
            status
                .map(|status| format!("{status:?}"))
                .unwrap_or_else(|| "n/a".to_string()),
            finding.rule_id,
            escape_table_cell(&finding.title),
            escape_table_cell(&evidence)
        ));
    }

    output.push('\n');
}

fn status_for_finding(report: &ReviewReport, finding: &Finding) -> Option<BaselineStatus> {
    report
        .summary
        .findings
        .iter()
        .position(|candidate| candidate == finding)
        .and_then(|index| report.finding_status(index))
        .and_then(|status| status.baseline_status)
}

fn render_ranges_suffix(file: &ChangedFile) -> String {
    let ranges = render_ranges(file);
    if ranges == "n/a" {
        String::new()
    } else {
        format!(" ({ranges})")
    }
}

fn render_ranges(file: &ChangedFile) -> String {
    if file.ranges.is_empty() {
        return "n/a".to_string();
    }

    file.ranges
        .iter()
        .map(|range| {
            if range.start == range.end {
                range.start.to_string()
            } else {
                format!("{}-{}", range.start, range.end)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
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
