mod console;
mod helpers;
mod json;
mod markdown;
mod sarif;

use crate::baseline::gate::CiGateResult;
use crate::output::{DetailLevel, FindingRenderLimit, OutputFormat};
use crate::review::ReviewSignalGateResult;
use crate::review::model::ReviewReport;

pub use console::render_console;
pub use json::render_json;
pub use markdown::render_markdown;
pub use sarif::render_review_sarif;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewRenderOptions {
    pub detail: DetailLevel,
    pub findings_limit: FindingRenderLimit,
}

impl ReviewRenderOptions {
    pub fn full() -> Self {
        Self {
            detail: DetailLevel::Full,
            findings_limit: FindingRenderLimit::Unlimited,
        }
    }
}

impl Default for ReviewRenderOptions {
    fn default() -> Self {
        Self {
            detail: DetailLevel::Findings,
            findings_limit: FindingRenderLimit::Default,
        }
    }
}

pub fn render(
    report: &ReviewReport,
    format: OutputFormat,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
) -> Result<String, serde_json::Error> {
    render_with_options(
        report,
        format,
        ci_gate,
        review_gate,
        ReviewRenderOptions::full(),
    )
}

pub fn render_with_options(
    report: &ReviewReport,
    format: OutputFormat,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
    options: ReviewRenderOptions,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(console::render_console_with_options(
            report,
            ci_gate,
            review_gate,
            options,
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
