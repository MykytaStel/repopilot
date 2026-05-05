use crate::scan::types::ScanSummary;

pub fn render(summary: &ScanSummary) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(summary)
}
