pub mod console;
pub mod html;
pub mod json;
pub mod markdown;

use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::scan::types::ScanSummary;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Console,
    Html,
    Json,
    Markdown,
}

pub fn render_scan_summary(
    summary: &ScanSummary,
    format: OutputFormat,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(console::render(summary)),
        OutputFormat::Html => Ok(html::render(summary)),
        OutputFormat::Json => json::render(summary),
        OutputFormat::Markdown => Ok(markdown::render(summary)),
    }
}

pub fn render_baseline_scan_report(
    report: &BaselineScanReport,
    format: OutputFormat,
    ci_gate: Option<&CiGateResult>,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(console::render_with_baseline(report, ci_gate)),
        OutputFormat::Html => Ok(html::render(&report.summary)),
        OutputFormat::Json => json::render_with_baseline(report, ci_gate),
        OutputFormat::Markdown => Ok(markdown::render(&report.summary)),
    }
}
