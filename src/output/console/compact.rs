use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::output::color;
use crate::output::report_stats::{
    ReportStats, build_report_stats, first_location, indexed_sorted_findings,
};
use crate::output::{FindingRenderLimit, RenderOptions};
use crate::scan::types::{DiagnosticSeverity, ScanSummary};
use std::fmt::Write;
use std::path::Path;

mod header;

pub(crate) fn render_with_options(summary: &ScanSummary, options: RenderOptions) -> String {
    let stats = build_report_stats(summary);
    let mut output = String::new();

    render_summary_header(&mut output, summary, &stats);
    render_findings_block(&mut output, summary, options.findings_limit);
    if !options.quiet {
        render_next_steps(&mut output, summary);
    }

    output
}

pub(crate) fn render_baseline_with_options(
    report: &BaselineScanReport,
    ci_gate: Option<&CiGateResult>,
    options: RenderOptions,
) -> String {
    let summary = &report.summary;
    let stats = build_report_stats(summary);
    let mut output = String::new();

    render_summary_header(&mut output, summary, &stats);
    render_baseline_block(&mut output, report, ci_gate);
    render_findings_block(&mut output, summary, options.findings_limit);
    if !options.quiet {
        render_next_steps(&mut output, summary);
    }

    output
}

fn render_summary_header(output: &mut String, summary: &ScanSummary, stats: &ReportStats) {
    let visible_count = visible_findings_count(summary);
    let status = status_label(summary, visible_count);
    let risk = compact_risk_label(stats);

    output.push_str("RepoPilot Scan\n\n");
    writeln!(output, "Status: {}", color::status_label(status)).unwrap();
    writeln!(output, "Risk: {}", color::risk_label(risk)).unwrap();
    writeln!(output, "Health: {}/100", stats.health_score).unwrap();
    writeln!(output, "Profile: {}", profile_label(summary)).unwrap();
    writeln!(
        output,
        "Path: {}",
        display_path(summary.root_path.as_path())
    )
    .unwrap();
    header::render_scope(output, summary);
    header::render_scope_accounting(output, summary);
    header::render_diagnostics_line(output, summary);
    header::render_local_feedback_line(output, summary);
    output.push('\n');
}

fn render_findings_block(
    output: &mut String,
    summary: &ScanSummary,
    findings_limit: FindingRenderLimit,
) {
    let visible_count = visible_findings_count(summary);
    writeln!(output, "Findings: {visible_count} visible").unwrap();
    writeln!(
        output,
        "Hidden suggestions: {} strict-only",
        summary.metrics.hidden_suggestions_count
    )
    .unwrap();
    output.push('\n');

    if summary.artifacts.findings.is_empty() {
        output.push_str("No visible risks found.\n\n");
        return;
    }

    output.push_str("Top findings:\n");
    let shown = findings_limit.compact_limit(summary.artifacts.findings.len());
    for (_, finding) in indexed_sorted_findings(&summary.artifacts.findings)
        .into_iter()
        .take(shown)
    {
        render_top_finding(output, finding);
    }
    if matches!(findings_limit, FindingRenderLimit::Limit(_))
        && shown < summary.artifacts.findings.len()
    {
        writeln!(
            output,
            "  showing {shown} of {} findings (--max-findings none shows all)",
            summary.artifacts.findings.len()
        )
        .unwrap();
    }
    output.push('\n');
}

fn render_top_finding(output: &mut String, finding: &Finding) {
    let priority = finding.risk.priority.label();
    let location = first_location(finding).unwrap_or_else(|| ".".to_string());
    writeln!(
        output,
        "  {:<3} {:<34} {}",
        priority, finding.rule_id, location
    )
    .unwrap();
}

fn render_baseline_block(
    output: &mut String,
    report: &BaselineScanReport,
    ci_gate: Option<&CiGateResult>,
) {
    match &report.baseline_path {
        Some(path) => writeln!(output, "Baseline: {}", path.display()).unwrap(),
        None => output.push_str("Baseline: none (all findings treated as new)\n"),
    }
    writeln!(output, "New findings: {}", report.new_count()).unwrap();
    writeln!(output, "Existing findings: {}", report.existing_count()).unwrap();
    if let Some(ci_gate) = ci_gate {
        let status = if ci_gate.passed() { "passed" } else { "failed" };
        writeln!(output, "CI gate: {status} ({})", ci_gate.label()).unwrap();
    }
    output.push('\n');
}

fn render_next_steps(output: &mut String, summary: &ScanSummary) {
    let path = command_path(summary.root_path.as_path());
    output.push_str("Next:\n");
    if summary.metrics.hidden_suggestions_count > 0 {
        writeln!(output, "  repopilot scan {path} --profile strict").unwrap();
    }
    writeln!(
        output,
        "  repopilot scan {path} --format markdown --output report.md"
    )
    .unwrap();
}

fn status_label(summary: &ScanSummary, visible_count: usize) -> &'static str {
    if summary.has_error_diagnostics() {
        "Scan completed with errors"
    } else if visible_count > 0 {
        "Attention needed"
    } else if summary
        .artifacts
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
    {
        "Scan completed with warnings"
    } else {
        "Clean"
    }
}

fn compact_risk_label(stats: &ReportStats) -> &'static str {
    match stats.risk_label {
        "Clean" | "Low" => "Low",
        "Moderate" => "Medium",
        "Elevated" | "High" => "High",
        other => other,
    }
}

fn visible_findings_count(summary: &ScanSummary) -> usize {
    if summary.metrics.visible_findings_count == 0 && !summary.artifacts.findings.is_empty() {
        summary.artifacts.findings.len()
    } else {
        summary.metrics.visible_findings_count
    }
}

fn profile_label(summary: &ScanSummary) -> &str {
    summary.visibility_profile.as_deref().unwrap_or("default")
}

fn display_path(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.display().to_string()
    }
}

fn command_path(path: &Path) -> String {
    let path = display_path(path);
    if path
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-'))
    {
        return path;
    }

    format!("'{}'", path.replace('\'', "'\\''"))
}
