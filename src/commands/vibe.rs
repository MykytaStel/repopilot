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
    budget: Option<String>,
    output: Option<PathBuf>,
    no_header: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_config = match config {
        Some(config_path) => load_optional_config(&config_path)?,
        None => load_default_config()?,
    };
    let scan_config = build_scan_config(&repo_config, None, None, None);

    let focus_category = match focus.as_deref() {
        Some(s) => match s.parse::<VibeCategory>() {
            Ok(c) => Some(c),
            Err(_) => {
                eprintln!(
                    "Warning: unknown focus '{}'. Expected: security, arch, quality, framework, all",
                    s
                );
                None
            }
        },
        None => None,
    };

    let budget_tokens = parse_budget(budget.as_deref()).unwrap_or(4096);

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

fn parse_budget(s: Option<&str>) -> Option<usize> {
    match s? {
        "2k" => Some(2048),
        "4k" => Some(4096),
        "8k" => Some(8192),
        "16k" => Some(16384),
        other => other.parse::<usize>().ok(),
    }
}
