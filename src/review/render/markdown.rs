use crate::baseline::diff::BaselineStatus;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::output::render_helpers::escape_table_cell;
use crate::review::model::ReviewReport;
use crate::review::render::helpers::{render_ranges, status_for_finding};
use crate::review::signals::tiered::ReviewSignal;

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
    if let Some(feedback) = &report.summary.local_feedback {
        output.push_str(&format!(
            "- **Local feedback:** {} suppression(s) loaded, {} finding(s) suppressed\n",
            feedback.suppressions_loaded, feedback.suppressed_findings_count
        ));
    }

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
    render_markdown_tiered_signals(&mut output, report);
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

/// Render the unified, confidence-tiered "Review Signals" section: one sub-table
/// per non-empty tier (definitely → maybe → noise). Boundary signals are folded
/// into the `definitely` tier rather than getting their own block.
fn render_markdown_tiered_signals(output: &mut String, report: &ReviewReport) {
    let tiered = &report.tiered_signals;
    if tiered.is_empty() {
        return;
    }

    output.push_str("## Review Signals (preview)\n\n");
    output.push_str(
        "Where to look first in this diff — flags, not verdicts. Open the report before merging.\n\n",
    );

    if report.boundary_missing_test {
        output.push_str(
            "> ⚠ A code boundary changed but no test did — confirm it's still covered.\n\n",
        );
    }

    render_markdown_tier(output, "Definitely sensitive", &tiered.definitely);
    render_markdown_tier(output, "Maybe sensitive", &tiered.maybe);
    render_markdown_tier(output, "Large diff / noise", &tiered.noise);
}

fn render_markdown_tier(output: &mut String, label: &str, signals: &[ReviewSignal]) {
    if signals.is_empty() {
        return;
    }

    output.push_str(&format!("### {label}\n\n"));
    output.push_str("| Signal | Location | Detail | Reach |\n");
    output.push_str("| --- | --- | --- | --- |\n");

    for signal in signals {
        let location = if signal.path.is_empty() {
            "—".to_string()
        } else {
            match signal.line {
                Some(line) => format!("`{}:{line}`", signal.path),
                None => format!("`{}`", signal.path),
            }
        };
        let detail = signal
            .detail
            .as_deref()
            .filter(|detail| !detail.is_empty())
            .map(escape_table_cell)
            .unwrap_or_else(|| "—".to_string());
        let reach = match signal.blast_radius {
            0 => "—".to_string(),
            count => format!("imported by {count}"),
        };
        output.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            escape_table_cell(&signal.headline),
            location,
            detail,
            reach
        ));
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

    output.push_str("| Severity | Confidence | Baseline | Rule | Title | Evidence |\n");
    output.push_str("| --- | --- | --- | --- | --- | --- |\n");

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
            "| {} | {} | {} | `{}` | {} | {} |\n",
            finding.severity_label(),
            finding.confidence_label(),
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
