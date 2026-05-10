mod console;
mod helpers;
mod json;
mod markdown;

use crate::baseline::gate::CiGateResult;
use crate::output::OutputFormat;
use crate::review::model::ReviewReport;

pub use console::render_console;
pub use json::render_json;
pub use markdown::render_markdown;

pub fn render(
    report: &ReviewReport,
    format: OutputFormat,
    ci_gate: Option<&CiGateResult>,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(render_console(report, ci_gate)),
        OutputFormat::Json => render_json(report, ci_gate),
        OutputFormat::Markdown => Ok(render_markdown(report, ci_gate)),
        OutputFormat::Html | OutputFormat::Sarif => {
            unreachable!("HTML and SARIF are not supported for the review command")
        }
    }
}
