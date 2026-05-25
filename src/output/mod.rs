pub mod ai_context;
pub mod ai_plan;
pub mod color;
pub mod console;
pub(crate) mod finding_helpers;
pub mod html;
pub mod json;
pub mod markdown;
pub mod prompt;
pub(crate) mod render_helpers;
pub(crate) mod report_stats;
pub(crate) mod report_text;
pub mod rules;
pub mod sarif;

use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::scan::types::ScanSummary;
use serde::Deserialize;

pub use color::{ColorChoice, ColorDestination};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Console,
    Html,
    Json,
    Markdown,
    Sarif,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleOutputStyle {
    Compact,
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindingRenderLimit {
    Default,
    Limit(usize),
    Unlimited,
}

impl FindingRenderLimit {
    pub fn compact_limit(self, total: usize) -> usize {
        match self {
            Self::Default => total.min(5),
            Self::Limit(limit) => total.min(limit),
            Self::Unlimited => total,
        }
    }

    pub fn detailed_limit(self, total: usize) -> usize {
        match self {
            Self::Default | Self::Unlimited => total,
            Self::Limit(limit) => total.min(limit),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderOptions {
    pub console_output_style: ConsoleOutputStyle,
    pub color_choice: ColorChoice,
    pub color_destination: ColorDestination,
    pub quiet: bool,
    pub findings_limit: FindingRenderLimit,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            console_output_style: ConsoleOutputStyle::Full,
            color_choice: ColorChoice::Auto,
            color_destination: ColorDestination::Stdout,
            quiet: false,
            findings_limit: FindingRenderLimit::Default,
        }
    }
}

impl RenderOptions {
    fn color_enabled(self) -> bool {
        color::resolve_color_enabled(self.color_choice, self.color_destination)
    }
}

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
