pub mod ai_context;
pub(crate) mod ai_plan;
pub mod color;
pub mod console;
pub mod decision_summary;
mod dispatch;
pub(crate) mod finding_helpers;
pub mod html;
pub mod json;
pub mod markdown;
pub(crate) mod render_helpers;
pub(crate) mod report_stats;
pub(crate) mod report_text;
pub mod sarif;

use serde::Deserialize;

pub use color::{ColorChoice, ColorDestination};
pub use dispatch::{
    render_baseline_scan_report, render_baseline_scan_report_with_options, render_scan_summary,
    render_scan_summary_with_options,
};

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
    Summary,
    Compact,
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailLevel {
    Summary,
    Findings,
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
