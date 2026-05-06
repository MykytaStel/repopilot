use crate::cli::CompareOutputFormatArg;
use repopilot::compare::diff::diff_summaries;
use repopilot::compare::render::render;
use repopilot::report::writer::write_report;
use repopilot::scan::types::ScanSummary;
use std::fs;
use std::path::PathBuf;

pub fn run(
    before: PathBuf,
    after: PathBuf,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let before_summary: ScanSummary =
        serde_json::from_str(&fs::read_to_string(&before)?)
            .map_err(|e| format!("Failed to parse {}: {e}", before.display()))?;

    let after_summary: ScanSummary = serde_json::from_str(&fs::read_to_string(&after)?)
        .map_err(|e| format!("Failed to parse {}: {e}", after.display()))?;

    let diff = diff_summaries(&before_summary, &after_summary);
    let rendered = render(&diff, format.into())?;

    write_report(&rendered, output.as_deref())?;

    Ok(())
}
