use crate::baseline::diff::BaselineStatus;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::output::render_helpers::escape_table_cell;
use crate::review::ReviewSignalGateResult;
use crate::review::model::ReviewReport;
use crate::review::render::helpers::{render_ranges, status_for_finding};
use crate::review::signals::tiered::ReviewSignal;

const REVIEW_SIGNAL_DETAIL_LIMIT: usize = 20;

pub fn render_markdown(report: &ReviewReport, ci_gate: Option<&CiGateResult>) -> String {
    render_markdown_with_gates(report, ci_gate, None)
}

pub fn render_markdown_with_gates(
    report: &ReviewReport,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
) -> String {
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
            "- **Local feedback:** {} finding + {} review suppression(s) loaded, {} finding(s) + {} review signal(s) suppressed\n",
            feedback.suppressions_loaded,
            feedback.review_suppressions_loaded,
            feedback.suppressed_findings_count,
            feedback.suppressed_review_signals_count,
        ));
    }

    if let Some(ci_gate) = ci_gate {
        let status = if ci_gate.passed() { "passed" } else { "failed" };
        output.push_str(&format!(
            "- **CI gate:** {status} (`{}`)\n",
            ci_gate.label()
        ));
    }
    if let Some(review_gate) = review_gate {
        let status = if review_gate.passed() {
            "passed"
        } else {
            "failed"
        };
        output.push_str(&format!(
            "- **Review gate:** {status} (`{}`, {} signal(s))\n",
            review_gate.label(),
            review_gate.failed_signals
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
    render_markdown_impact_paths(&mut output, report);
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

/// Bounded-depth dependency impact: per changed file, its direct and
/// transitive dependents up to the configured depth, plus a rolled-up
/// affected-surface summary. Additive to (and independent of) blast radius.
fn render_markdown_impact_paths(output: &mut String, report: &ReviewReport) {
    let impact = &report.impact_paths;
    if impact.files.is_empty() {
        return;
    }

    output.push_str("## Impact Paths\n\n");
    output.push_str(&format!(
        "{} file(s) across {} director{} may be affected (depth {}).\n\n",
        impact.affected_surface.impacted_files,
        impact.affected_surface.affected_directories.len(),
        if impact.affected_surface.affected_directories.len() == 1 {
            "y"
        } else {
            "ies"
        },
        impact.depth,
    ));

    let mut remaining = REVIEW_SIGNAL_DETAIL_LIMIT;
    'files: for file_impact in &impact.files {
        output.push_str(&format!("- `{}`\n", file_impact.path.display()));
        for dependent in &file_impact.direct_dependents {
            if remaining == 0 {
                output.push_str("  - ... (truncated)\n");
                break 'files;
            }
            output.push_str(&format!("  - direct: `{}`\n", dependent.display()));
            remaining -= 1;
        }
        for dependent in &file_impact.transitive_dependents {
            if remaining == 0 {
                output.push_str("  - ... (truncated)\n");
                break 'files;
            }
            output.push_str(&format!("  - transitive: `{}`\n", dependent.display()));
            remaining -= 1;
        }
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

    let mut remaining = REVIEW_SIGNAL_DETAIL_LIMIT;
    render_markdown_tier(
        output,
        "Definitely sensitive",
        &tiered.definitely,
        &mut remaining,
    );
    render_markdown_tier(output, "Maybe sensitive", &tiered.maybe, &mut remaining);
    render_markdown_tier(output, "Large diff / noise", &tiered.noise, &mut remaining);
    if tiered.has_taint_signal() {
        output.push_str(
            "> Taint signals trace input → sink reachability — a path exists, not a confirmed vulnerability. Verify before acting.\n\n",
        );
    }
}

fn render_markdown_tier(
    output: &mut String,
    label: &str,
    signals: &[ReviewSignal],
    remaining: &mut usize,
) {
    let active = signals
        .iter()
        .filter(|signal| !signal.suppressed)
        .collect::<Vec<_>>();
    if active.is_empty() {
        return;
    }

    output.push_str(&format!("### {label}\n\n"));
    output.push_str("| Signal | Location | Detail | Reach | Verification |\n");
    output.push_str("| --- | --- | --- | --- | --- |\n");

    let shown = active.len().min(*remaining);
    for signal in active.iter().take(shown) {
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
        let verification = signal
            .verification_plan
            .as_ref()
            .and_then(|plan| plan.steps.first())
            .map(|step| escape_table_cell(step))
            .unwrap_or_else(|| "—".to_string());
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            escape_table_cell(&signal.headline),
            location,
            detail,
            reach,
            verification
        ));
    }

    *remaining = remaining.saturating_sub(shown);
    if active.len() > shown {
        output.push_str(&format!(
            "\n_{} additional signal(s) omitted; use JSON for the full list._\n",
            active.len() - shown
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
                let snippet = crate::findings::redaction::human_evidence_snippet(
                    finding,
                    evidence.snippet.trim(),
                );
                format!(
                    "`{}:{}` - {}",
                    evidence.path.display(),
                    evidence.line_start,
                    snippet
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
