use crate::commands::CliExit;
use crate::commands::scan::{finish_spinner, make_spinner};
use crate::commands::{ScanConfigOverrides, build_scan_config};
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::output::harden::{HardenOptions, render as render_harden};
use repopilot::output::vibe::VibeCategory;
use repopilot::report::writer::write_report;
use repopilot::scan::scanner::scan_path_with_config;
use std::path::PathBuf;

pub fn run(
    path: PathBuf,
    config: Option<PathBuf>,
    focus: Option<String>,
    budget: Option<usize>,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_config = match config {
        Some(config_path) => load_optional_config(&config_path)?,
        None => load_default_config()?,
    };
    let scan_config = build_scan_config(&repo_config, ScanConfigOverrides::default());
    let focus_category = parse_focus(focus.as_deref())?;

    let pb = make_spinner();
    let summary = scan_path_with_config(&path, &scan_config)?;
    finish_spinner(pb);

    let rendered = render_harden(
        &summary,
        &HardenOptions {
            focus: focus_category,
            budget_tokens: budget.unwrap_or(4096),
        },
    );
    write_report(&rendered, output.as_deref())?;

    Ok(())
}

fn parse_focus(focus: Option<&str>) -> Result<Option<VibeCategory>, Box<dyn std::error::Error>> {
    match focus {
        Some(value) => Ok(Some(value.parse::<VibeCategory>().map_err(|_| CliExit {
            code: 2,
            message: format!(
                "Invalid focus '{value}'. Expected: security, arch, architecture, quality, framework, all"
            ),
        })?)),
        None => Ok(None),
    }
}
