use crate::commands::focus::parse_focus_category;
use crate::commands::progress::{finish_spinner, make_spinner};
use crate::commands::scan_config::{ScanConfigOverrides, build_scan_config};
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::output::vibe::{DEFAULT_TOKEN_BUDGET, VibeCategory};
use repopilot::report::writer::write_report;
use repopilot::scan::scanner::scan_path_with_config;
use repopilot::scan::types::ScanSummary;
use std::path::PathBuf;

pub struct LlmCommandArgs {
    pub path: PathBuf,
    pub config: Option<PathBuf>,
    pub focus: Option<String>,
    pub budget: Option<usize>,
    pub output: Option<PathBuf>,
}

pub fn run_markdown_command<F>(
    args: LlmCommandArgs,
    render: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&ScanSummary, Option<VibeCategory>, usize) -> String,
{
    let repo_config = match args.config {
        Some(config_path) => load_optional_config(&config_path)?,
        None => load_default_config()?,
    };
    let scan_config = build_scan_config(&repo_config, ScanConfigOverrides::default());
    let focus_category = parse_focus_category(args.focus.as_deref())?;
    let budget_tokens = args.budget.unwrap_or(DEFAULT_TOKEN_BUDGET);

    let pb = make_spinner("Scanning...");
    let summary = scan_path_with_config(&args.path, &scan_config)?;
    finish_spinner(pb);

    let rendered = render(&summary, focus_category, budget_tokens);
    write_report(&rendered, args.output.as_deref())?;

    Ok(())
}
