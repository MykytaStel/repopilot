mod baseline;
mod findings;
mod react_native;
mod sections;
mod workspace;

use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::output::RenderOptions;
use crate::output::markdown::baseline::render_baseline_section;
use crate::output::markdown::findings::{render_findings_index, render_grouped_findings};
use crate::output::markdown::react_native::render_react_native_section;
use crate::output::markdown::sections::{
    render_framework_projects_section, render_frameworks_section, render_languages_section,
    render_overview, render_risk_summary, render_signal_quality, render_top_risk_clusters,
    render_top_rules,
};
use crate::output::markdown::workspace::render_workspace_risk_table;
use crate::output::report_stats::build_report_stats;
use crate::scan::types::ScanSummary;

pub fn render(summary: &ScanSummary) -> String {
    render_with_options(summary, RenderOptions::default())
}

pub fn render_with_options(summary: &ScanSummary, options: RenderOptions) -> String {
    let stats = build_report_stats(summary);
    let mut output = String::new();

    output.push_str("# RepoPilot Scan Report\n\n");
    render_overview(&mut output, summary, &stats);
    render_risk_summary(&mut output, summary, &stats);
    render_signal_quality(&mut output, summary);
    render_top_risk_clusters(&mut output, &summary.artifacts.findings);
    render_top_rules(&mut output, &stats);
    render_languages_section(&mut output, summary);
    render_frameworks_section(&mut output, &summary.artifacts.detected_frameworks);
    render_framework_projects_section(&mut output, &summary.artifacts.framework_projects);
    if let Some(rn) = &summary.artifacts.react_native {
        render_react_native_section(&mut output, rn);
    }
    render_workspace_risk_table(&mut output, &summary.artifacts.findings);
    render_findings_index(
        &mut output,
        &summary.artifacts.findings,
        None,
        options.findings_limit,
    );
    render_grouped_findings(
        &mut output,
        &summary.artifacts.findings,
        options.findings_limit,
        |_| None,
    );

    output
}

pub fn render_with_baseline(report: &BaselineScanReport, ci_gate: Option<&CiGateResult>) -> String {
    render_baseline_with_options(report, ci_gate, RenderOptions::default())
}

pub fn render_baseline_with_options(
    report: &BaselineScanReport,
    ci_gate: Option<&CiGateResult>,
    options: RenderOptions,
) -> String {
    let summary = &report.summary;
    let stats = build_report_stats(summary);
    let mut output = String::new();

    output.push_str("# RepoPilot Scan Report\n\n");
    render_overview(&mut output, summary, &stats);
    render_baseline_section(&mut output, report, ci_gate);
    render_risk_summary(&mut output, summary, &stats);
    render_signal_quality(&mut output, summary);
    render_top_risk_clusters(&mut output, &summary.artifacts.findings);
    render_top_rules(&mut output, &stats);
    render_languages_section(&mut output, summary);
    render_frameworks_section(&mut output, &summary.artifacts.detected_frameworks);
    render_framework_projects_section(&mut output, &summary.artifacts.framework_projects);
    if let Some(rn) = &summary.artifacts.react_native {
        render_react_native_section(&mut output, rn);
    }
    render_workspace_risk_table(&mut output, &summary.artifacts.findings);
    render_findings_index(
        &mut output,
        &summary.artifacts.findings,
        Some(report),
        options.findings_limit,
    );
    render_grouped_findings(
        &mut output,
        &summary.artifacts.findings,
        options.findings_limit,
        |index| Some(report.finding_status(index).lowercase_label()),
    );

    output
}
