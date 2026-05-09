use crate::commands::CliExit;
use crate::commands::build_scan_config;
use crate::commands::scan::{finish_spinner, make_spinner};
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::output::vibe::{VibeCategory, VibeOptions, render as render_vibe};
use repopilot::report::writer::write_report;
use repopilot::scan::scanner::scan_path_with_config;
use std::path::PathBuf;

pub fn run(
    path: PathBuf,
    config: Option<PathBuf>,
    focus: Option<String>,
    budget: Option<usize>,
    output: Option<PathBuf>,
    no_header: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_config = match config {
        Some(config_path) => load_optional_config(&config_path)?,
        None => load_default_config()?,
    };
    let scan_config = build_scan_config(&repo_config, None, None, None);

    let focus_category = match focus.as_deref() {
        Some(s) => Some(s.parse::<VibeCategory>().map_err(|_| {
            CliExit {
                code: 2,
                message: format!(
                    "Invalid focus '{s}'. Expected: security, arch, architecture, quality, framework, all"
                ),
            }
        })?),
        None => None,
    };

    let budget_tokens = budget.unwrap_or(4096);

    let pb = make_spinner();
    let summary = scan_path_with_config(&path, &scan_config)?;
    finish_spinner(pb);

    let opts = VibeOptions {
        focus: focus_category,
        budget_tokens,
        no_header,
    };

    let rendered = render_vibe(&summary, &opts);
    write_report(&rendered, output.as_deref())?;

    Ok(())
}
