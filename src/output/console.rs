mod baseline;
mod compact;
mod findings;
mod react_native;
mod sections;
mod workspace;

use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::output::console::baseline::render_baseline_block;
use crate::output::console::findings::render_grouped_findings;
use crate::output::console::react_native::render_react_native_section;
use crate::output::console::sections::{
    render_framework_projects_section, render_frameworks_section, render_header,
    render_languages_section, render_risk_summary, render_signal_quality, render_top_risk_clusters,
    render_top_rules,
};
use crate::output::console::workspace::workspace_risk_table;
use crate::output::report_stats::build_report_stats;
use crate::output::{ConsoleOutputStyle, RenderOptions};
use crate::scan::types::ScanSummary;

pub fn render(summary: &ScanSummary) -> String {
    render_full(summary)
}

pub fn render_with_options(summary: &ScanSummary, options: RenderOptions) -> String {
    match options.console_output_style {
        ConsoleOutputStyle::Summary => compact::render_summary_with_options(summary, options),
        ConsoleOutputStyle::Compact => compact::render_with_options(summary, options),
        ConsoleOutputStyle::Full => render_full_with_options(summary, options),
    }
}

fn render_full(summary: &ScanSummary) -> String {
    render_full_with_options(summary, RenderOptions::default())
}

fn render_full_with_options(summary: &ScanSummary, options: RenderOptions) -> String {
    let stats = build_report_stats(summary);
    let mut output = String::new();

    render_header(&mut output, summary, &stats);
    render_risk_summary(&mut output, &stats);
    render_signal_quality(&mut output, summary);
    render_top_risk_clusters(&mut output, &summary.artifacts.findings);
    render_top_rules(&mut output, &stats);
    render_languages_section(&mut output, summary);
    render_frameworks_section(&mut output, &summary.artifacts.detected_frameworks);
    render_framework_projects_section(&mut output, &summary.artifacts.framework_projects);
    if let Some(rn) = &summary.artifacts.react_native {
        render_react_native_section(&mut output, rn);
    }
    workspace_risk_table(&mut output, &summary.artifacts.findings);
    render_grouped_findings(
        &mut output,
        &summary.artifacts.findings,
        options.findings_limit,
        |_| None,
    );

    output
}

pub fn render_with_baseline(report: &BaselineScanReport, ci_gate: Option<&CiGateResult>) -> String {
    render_full_with_baseline(report, ci_gate)
}

pub fn render_baseline_with_options(
    report: &BaselineScanReport,
    ci_gate: Option<&CiGateResult>,
    options: RenderOptions,
) -> String {
    match options.console_output_style {
        ConsoleOutputStyle::Summary => {
            compact::render_baseline_summary_with_options(report, ci_gate, options)
        }
        ConsoleOutputStyle::Compact => {
            compact::render_baseline_with_options(report, ci_gate, options)
        }
        ConsoleOutputStyle::Full => render_full_baseline_with_options(report, ci_gate, options),
    }
}

fn render_full_with_baseline(
    report: &BaselineScanReport,
    ci_gate: Option<&CiGateResult>,
) -> String {
    render_full_baseline_with_options(report, ci_gate, RenderOptions::default())
}

fn render_full_baseline_with_options(
    report: &BaselineScanReport,
    ci_gate: Option<&CiGateResult>,
    options: RenderOptions,
) -> String {
    let summary = &report.summary;
    let stats = build_report_stats(summary);
    let mut output = String::new();

    render_header(&mut output, summary, &stats);
    render_baseline_block(&mut output, report, ci_gate);
    render_risk_summary(&mut output, &stats);
    render_signal_quality(&mut output, summary);
    render_top_risk_clusters(&mut output, &summary.artifacts.findings);
    render_top_rules(&mut output, &stats);
    render_languages_section(&mut output, summary);
    render_frameworks_section(&mut output, &summary.artifacts.detected_frameworks);
    render_framework_projects_section(&mut output, &summary.artifacts.framework_projects);
    if let Some(rn) = &summary.artifacts.react_native {
        render_react_native_section(&mut output, rn);
    }
    workspace_risk_table(&mut output, &summary.artifacts.findings);
    render_grouped_findings(
        &mut output,
        &summary.artifacts.findings,
        options.findings_limit,
        |index| Some(report.finding_status(index).lowercase_label()),
    );

    output
}
