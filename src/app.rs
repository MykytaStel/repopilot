use crate::cli::{Cli, Commands, OutputFormatArg};
use repopilot::compare::diff::diff_summaries;
use repopilot::compare::render::{render_console as compare_console, render_markdown as compare_markdown};
use repopilot::output::render_scan_summary;
use repopilot::report::writer::write_report;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use repopilot::scan::types::ScanSummary;
use std::fs;

pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Scan {
            path,
            format,
            output,
            max_file_loc,
            max_directory_modules,
            max_directory_depth,
        } => {
            let config =
                build_scan_config(max_file_loc, max_directory_modules, max_directory_depth);
            let summary = scan_path_with_config(&path, &config)?;
            let rendered_report = render_scan_summary(&summary, format.into())?;

            write_report(&rendered_report, output.as_deref())?;

            Ok(())
        }

        Commands::Compare {
            before,
            after,
            format,
            output,
        } => {
            let before_summary: ScanSummary =
                serde_json::from_str(&fs::read_to_string(&before)?).map_err(|e| {
                    format!("Failed to parse {}: {e}", before.display())
                })?;

            let after_summary: ScanSummary =
                serde_json::from_str(&fs::read_to_string(&after)?).map_err(|e| {
                    format!("Failed to parse {}: {e}", after.display())
                })?;

            let diff = diff_summaries(&before_summary, &after_summary);

            let rendered = match format {
                OutputFormatArg::Markdown => compare_markdown(&diff),
                _ => compare_console(&diff),
            };

            write_report(&rendered, output.as_deref())?;

            Ok(())
        }
    }
}

fn build_scan_config(
    max_file_loc: Option<usize>,
    max_directory_modules: Option<usize>,
    max_directory_depth: Option<usize>,
) -> ScanConfig {
    let mut config = ScanConfig::default();

    if let Some(threshold) = max_file_loc {
        config = config.with_large_file_loc_threshold(threshold);
    }

    if let Some(modules) = max_directory_modules {
        config.max_directory_modules = modules;
    }

    if let Some(depth) = max_directory_depth {
        config.max_directory_depth = depth;
    }

    config
}

