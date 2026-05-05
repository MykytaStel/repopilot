use crate::cli::{Cli, Commands, CompareOutputFormatArg};
use repopilot::compare::diff::diff_summaries;
use repopilot::compare::render::{
    render_console as compare_console, render_json as compare_json,
    render_markdown as compare_markdown,
};
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::config::model::RepoPilotConfig;
use repopilot::config::template::default_config_toml;
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
            config,
            max_file_loc,
            max_directory_modules,
            max_directory_depth,
        } => {
            let repo_config = match config {
                Some(config_path) => load_optional_config(&config_path)?,
                None => load_default_config()?,
            };
            let scan_config = build_scan_config(
                &repo_config,
                max_file_loc,
                max_directory_modules,
                max_directory_depth,
            );
            let output_format = format
                .map(Into::into)
                .unwrap_or(repo_config.output.default_format);
            let summary = scan_path_with_config(&path, &scan_config)?;
            let rendered_report = render_scan_summary(&summary, output_format)?;

            write_report(&rendered_report, output.as_deref())?;

            Ok(())
        }

        Commands::Init { force, path } => {
            if path.exists() && !force {
                println!(
                    "Config already exists at {}. Use `repopilot init --force` to overwrite it.",
                    path.display()
                );
                return Ok(());
            }

            fs::write(&path, default_config_toml())?;
            println!("Created RepoPilot config at {}", path.display());

            Ok(())
        }

        Commands::Compare {
            before,
            after,
            format,
            output,
        } => {
            let before_summary: ScanSummary =
                serde_json::from_str(&fs::read_to_string(&before)?)
                    .map_err(|e| format!("Failed to parse {}: {e}", before.display()))?;

            let after_summary: ScanSummary = serde_json::from_str(&fs::read_to_string(&after)?)
                .map_err(|e| format!("Failed to parse {}: {e}", after.display()))?;

            let diff = diff_summaries(&before_summary, &after_summary);

            let rendered = match format {
                CompareOutputFormatArg::Console => Ok(compare_console(&diff)),
                CompareOutputFormatArg::Json => compare_json(&diff),
                CompareOutputFormatArg::Markdown => Ok(compare_markdown(&diff)),
            }?;

            write_report(&rendered, output.as_deref())?;

            Ok(())
        }
    }
}

fn build_scan_config(
    repo_config: &RepoPilotConfig,
    max_file_loc: Option<usize>,
    max_directory_modules: Option<usize>,
    max_directory_depth: Option<usize>,
) -> ScanConfig {
    let mut config = repo_config.to_scan_config();

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
