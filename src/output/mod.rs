pub mod console;
pub mod html;
pub mod json;
pub mod markdown;

use crate::scan::types::ScanSummary;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
