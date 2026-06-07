mod console;
mod helpers;
mod json;
mod markdown;
mod sarif;

use crate::baseline::gate::CiGateResult;
use crate::output::OutputFormat;
use crate::review::ReviewSignalGateResult;
use crate::review::model::ReviewReport;

pub use console::render_console;
pub use json::render_json;
pub use markdown::render_markdown;
pub use sarif::render_review_sarif;

pub fn render(
    report: &ReviewReport,
    format: OutputFormat,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(console::render_console_with_gates(
            report,
            ci_gate,
            review_gate,
        )),
        OutputFormat::Json => json::render_json_with_gates(report, ci_gate, review_gate),
        OutputFormat::Markdown => Ok(markdown::render_markdown_with_gates(
            report,
            ci_gate,
            review_gate,
        )),
        OutputFormat::Html | OutputFormat::Sarif => {
            unreachable!("HTML and SARIF are not supported for the review command")
        }
    }
}
