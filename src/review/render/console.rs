use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::review::ReviewSignalGateResult;
use crate::review::model::ReviewReport;
use crate::review::render::helpers::render_ranges_suffix;
use crate::review::signals::tiered::ReviewSignal;

const REVIEW_SIGNAL_DETAIL_LIMIT: usize = 20;

pub fn render_console(report: &ReviewReport, ci_gate: Option<&CiGateResult>) -> String {
    render_console_with_gates(report, ci_gate, None)
}

pub fn render_console_with_gates(
    report: &ReviewReport,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
) -> String {
    let mut output = String::new();

    output.push_str("RepoPilot Review\n");
    output.push_str(&format!("Path: {}\n", report.summary.root_path.display()));
    output.push_str(&format!("Git root: {}\n", report.repo_root.display()));
    match &report.baseline_path {
        Some(path) => output.push_str(&format!("Baseline: {}\n", path.display())),
        None => output.push_str("Baseline: none (all findings treated as new)\n"),
    }
    if let Some(feedback) = &report.summary.local_feedback {
        output.push_str(&format!(
            "Local feedback: {} finding + {} review suppression(s) loaded, {} finding(s) + {} review signal(s) suppressed\n",
            feedback.suppressions_loaded,
            feedback.review_suppressions_loaded,
            feedback.suppressed_findings_count,
            feedback.suppressed_review_signals_count,
        ));
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
    if let Some(review_gate) = review_gate {
        let status = if review_gate.passed() {
            "passed"
        } else {
            "failed"
        };
        output.push_str(&format!(
            "Review gate: {status} ({}, {} signal(s))\n",
            review_gate.label(),
            review_gate.failed_signals
        ));
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
    render_tiered_signals(&mut output, report);
    render_findings_group(&mut output, "In-diff findings", &report.in_diff_findings());
    render_findings_group(
        &mut output,
        "Out-of-diff findings",
        &report.out_of_diff_findings(),
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

/// Render the unified, confidence-tiered "Review signals" block. Boundary,
/// behavioral, algorithmic, and taint signals are grouped into three tiers so
/// the eye goes to the riskiest part of the diff first. The old standalone
/// boundary block is folded into the `definitely` tier.
fn render_tiered_signals(output: &mut String, report: &ReviewReport) {
    let tiered = &report.tiered_signals;
    if tiered.is_empty() {
        return;
    }

    output.push_str("\nReview signals [preview]:\n");
    output.push_str(
        "  Where to look first in this diff \u{2014} flags, not verdicts. Open the report before merging.\n",
    );

    let mut remaining = REVIEW_SIGNAL_DETAIL_LIMIT;
    render_tier_group(
        output,
        "Definitely sensitive",
        &tiered.definitely,
        &mut remaining,
    );
    if report.boundary_missing_test {
        output.push_str(
            "    \u{26a0} A code boundary changed but no test did \u{2014} confirm it's still covered.\n",
        );
    }
    render_tier_group(output, "Maybe sensitive", &tiered.maybe, &mut remaining);
    render_tier_group(output, "Large diff / noise", &tiered.noise, &mut remaining);
    if tiered.has_taint_signal() {
        output.push_str(
            "  Taint signals trace input \u{2192} sink reachability \u{2014} a path exists, not a confirmed vulnerability. Verify before acting.\n",
        );
    }
}

fn render_tier_group(
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

    output.push_str(&format!("  {label}:\n"));
    let shown = active.len().min(*remaining);
    for signal in active.iter().take(shown) {
        output.push_str(&format!(
            "    \u{2691} {}{}{}{}\n",
            signal.headline,
            render_signal_location(signal),
            render_signal_detail(signal),
            render_reach_suffix(signal.blast_radius),
        ));
    }
    *remaining = remaining.saturating_sub(shown);
    if active.len() > shown {
        output.push_str(&format!(
            "    ... {} additional signal(s) omitted; use JSON for the full list\n",
            active.len() - shown
        ));
    }
}

fn render_signal_location(signal: &ReviewSignal) -> String {
    if signal.path.is_empty() {
        return String::new();
    }
    match signal.line {
        Some(line) => format!(" \u{2014} {}:{line}", signal.path),
        None => format!(" \u{2014} {}", signal.path),
    }
}

fn render_signal_detail(signal: &ReviewSignal) -> String {
    match &signal.detail {
        Some(detail) if !detail.is_empty() => format!("  {detail}"),
        _ => String::new(),
    }
}

fn render_reach_suffix(blast_radius: usize) -> String {
    match blast_radius {
        0 => String::new(),
        1 => "  (imported by 1 file)".to_string(),
        count => format!("  (imported by {count} files)"),
    }
}

fn render_findings_group(output: &mut String, label: &str, findings: &[&Finding]) {
    output.push_str(&format!("\n{label}: {}\n", findings.len()));

    if findings.is_empty() {
        return;
    }

    for finding in findings {
        output.push_str(&format!(
            "  [{} confidence={}] {} - {}\n",
            finding.severity_label(),
            finding.confidence_label(),
            finding.rule_id,
            finding.title
        ));

        for evidence in &finding.evidence {
            output.push_str(&format!(
                "    Evidence: {}:{} - {}\n",
                evidence.path.display(),
                evidence.line_start,
                evidence.snippet.trim()
            ));
        }
    }
}
