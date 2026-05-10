use crate::baseline::diff::BaselineStatus;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::output::render_helpers::escape_table_cell;
use crate::review::model::ReviewReport;
use crate::review::render::helpers::{render_ranges, status_for_finding};

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
                    "`{}:{}` - {}",
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
