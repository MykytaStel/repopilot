pub mod console;
pub mod json;

use crate::scan::types::ScanSummary;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Console,
    Json,
}

pub fn render_scan_summary(
    summary: &ScanSummary,
    format: OutputFormat,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(console::render(summary)),
        OutputFormat::Json => json::render(summary),
    }
}
