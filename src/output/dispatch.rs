use super::{OutputFormat, RenderOptions, color, console, html, json, markdown, sarif};
use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::scan::types::ScanSummary;

pub fn render_scan_summary(
    summary: &ScanSummary,
    format: OutputFormat,
) -> Result<String, serde_json::Error> {
    render_scan_summary_with_options(summary, format, RenderOptions::default())
}

pub fn render_scan_summary_with_options(
    summary: &ScanSummary,
    format: OutputFormat,
    options: RenderOptions,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(color::with_color_enabled(options.color_enabled(), || {
            console::render_with_options(summary, options)
        })),
        OutputFormat::Html => Ok(html::render(summary)),
        OutputFormat::Json => json::render(summary),
        OutputFormat::Markdown => Ok(markdown::render_with_options(summary, options)),
        OutputFormat::Sarif => sarif::render(summary),
    }
}

pub fn render_baseline_scan_report(
    report: &BaselineScanReport,
    format: OutputFormat,
    ci_gate: Option<&CiGateResult>,
) -> Result<String, serde_json::Error> {
    render_baseline_scan_report_with_options(report, format, ci_gate, RenderOptions::default())
}

pub fn render_baseline_scan_report_with_options(
    report: &BaselineScanReport,
    format: OutputFormat,
    ci_gate: Option<&CiGateResult>,
    options: RenderOptions,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(color::with_color_enabled(options.color_enabled(), || {
            console::render_baseline_with_options(report, ci_gate, options)
        })),
        OutputFormat::Html => Ok(html::render_with_baseline(report, ci_gate)),
        OutputFormat::Json => json::render_with_baseline(report, ci_gate),
        OutputFormat::Markdown => Ok(markdown::render_baseline_with_options(
            report, ci_gate, options,
        )),
        OutputFormat::Sarif => sarif::render_with_baseline(report),
    }
}

impl RenderOptions {
    fn color_enabled(self) -> bool {
        color::resolve_color_enabled(self.color_choice, self.color_destination)
    }
}
