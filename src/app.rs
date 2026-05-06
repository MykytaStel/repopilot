use crate::cli::{BaselineCommands, Cli, Commands, CompareOutputFormatArg};
use chrono::{SecondsFormat, Utc};
use repopilot::baseline::diff::{all_findings_new, diff_summary_against_baseline};
use repopilot::baseline::gate::evaluate_ci_gate;
use repopilot::baseline::key::normalized_relative_path;
use repopilot::baseline::model::Baseline;
use repopilot::baseline::reader::read_baseline;
use repopilot::baseline::writer::{BaselineWriteError, write_baseline};
use repopilot::compare::diff::diff_summaries;
use repopilot::compare::render::{
    render_console as compare_console, render_json as compare_json,
    render_markdown as compare_markdown,
};
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::config::model::RepoPilotConfig;
use repopilot::config::template::default_config_toml;
use repopilot::findings::types::Severity;
use repopilot::output::{render_baseline_scan_report, render_scan_summary};
use repopilot::report::writer::write_report;
use repopilot::review::render::{
    render_console as review_console, render_json as review_json,
    render_markdown as review_markdown,
};
use repopilot::review::{build_review_report, review_report_for_ci};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use repopilot::scan::types::ScanSummary;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Baseline { command } => match command {
            BaselineCommands::Create {
                path,
                output,
                force,
            } => {
                let output_path = output.unwrap_or_else(|| default_baseline_path(&path));

                if output_path.exists() && !force {
                    println!(
                        "Baseline already exists at {}. Use `repopilot baseline create {} --force` to overwrite it.",
                        output_path.display(),
                        path.display()
                    );
                    return Ok(());
                }

                let repo_config = load_default_config()?;
                let scan_config = repo_config.to_scan_config();
                let summary = scan_path_with_config(&path, &scan_config)?;
                let baseline = Baseline::from_scan_summary(
                    &summary,
                    &summary.root_path,
                    display_baseline_root(&path),
                    current_timestamp(),
                );

                match write_baseline(&baseline, &output_path, force) {
                    Ok(()) => {
                        print_baseline_create_summary(&path, &output_path, &summary);
                        Ok(())
                    }
                    Err(BaselineWriteError::AlreadyExists { .. }) => {
                        println!(
                            "Baseline already exists at {}. Use `repopilot baseline create {} --force` to overwrite it.",
                            output_path.display(),
                            path.display()
                        );
                        Ok(())
                    }
                    Err(error) => Err(Box::new(error)),
                }
            }
        },

        Commands::Scan {
            path,
            format,
            output,
            config,
            baseline,
            fail_on,
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

            if baseline.is_some() || fail_on.is_some() {
                let baseline_report = match baseline {
                    Some(baseline_path) => {
                        let baseline_file = read_baseline(&baseline_path)?;
                        diff_summary_against_baseline(summary, &baseline_file, baseline_path)
                    }
                    None => all_findings_new(summary),
                };

                let ci_gate = fail_on
                    .map(Into::into)
                    .map(|fail_on| evaluate_ci_gate(&baseline_report, fail_on));
                let rendered_report =
                    render_baseline_scan_report(&baseline_report, output_format, ci_gate.as_ref())?;

                write_report(&rendered_report, output.as_deref())?;

                if let Some(ci_gate) = ci_gate
                    && let Some(message) = ci_gate.failure_message()
                {
                    return Err(Box::new(CliExit { code: 1, message }));
                }

                return Ok(());
            }

            let rendered_report = render_scan_summary(&summary, output_format)?;

            write_report(&rendered_report, output.as_deref())?;

            Ok(())
        }

        Commands::Review {
            path,
            base,
            head,
            config,
            baseline,
            fail_on,
            format,
            output,
            max_file_loc,
            max_directory_modules,
            max_directory_depth,
        } => {
            if base.is_none() && head.is_some() {
                return Err(Box::new(CliExit {
                    code: 1,
                    message: "`repopilot review --head` requires --base".to_string(),
                }));
            }

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
            let summary = scan_path_with_config(&path, &scan_config)?;
            let baseline_file = match baseline {
                Some(baseline_path) => Some((read_baseline(&baseline_path)?, baseline_path)),
                None => None,
            };
            let baseline_ref = baseline_file
                .as_ref()
                .map(|(baseline, path)| (baseline, path.clone()));
            let review_report = build_review_report(
                summary,
                &path,
                base.as_deref(),
                head.as_deref(),
                baseline_ref,
            )?;
            let ci_report = review_report_for_ci(&review_report);
            let ci_gate = fail_on
                .map(Into::into)
                .map(|fail_on| evaluate_ci_gate(&ci_report, fail_on));
            let rendered_report = match format {
                CompareOutputFormatArg::Console => {
                    Ok(review_console(&review_report, ci_gate.as_ref()))
                }
                CompareOutputFormatArg::Json => review_json(&review_report, ci_gate.as_ref()),
                CompareOutputFormatArg::Markdown => {
                    Ok(review_markdown(&review_report, ci_gate.as_ref()))
                }
            }?;

            write_report(&rendered_report, output.as_deref())?;

            if let Some(ci_gate) = ci_gate
                && let Some(message) = ci_gate.failure_message()
            {
                return Err(Box::new(CliExit { code: 1, message }));
            }

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

#[derive(Debug)]
pub struct CliExit {
    pub code: i32,
    pub message: String,
}

impl fmt::Display for CliExit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for CliExit {}

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

fn default_baseline_path(scan_path: &Path) -> PathBuf {
    if scan_path == Path::new(".") {
        return PathBuf::from(".repopilot/baseline.json");
    }

    if scan_path.is_file() {
        return scan_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(".repopilot/baseline.json");
    }

    scan_path.join(".repopilot/baseline.json")
}

fn display_baseline_root(path: &Path) -> String {
    if path.is_absolute()
        && let Ok(current_dir) = std::env::current_dir()
    {
        return normalized_relative_path(path, &current_dir);
    }

    normalized_relative_path(path, Path::new("."))
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn print_baseline_create_summary(scan_path: &Path, output_path: &Path, summary: &ScanSummary) {
    let counts = SeverityCounts::from_summary(summary);

    println!("RepoPilot Baseline");
    println!();
    println!("Scanned path: {}", scan_path.display());
    println!("Baseline written to: {}", output_path.display());
    println!();
    println!("Stored findings:");
    println!("- Critical: {}", counts.critical);
    println!("- High: {}", counts.high);
    println!("- Medium: {}", counts.medium);
    println!("- Low: {}", counts.low);
    println!("- Info: {}", counts.info);
}

#[derive(Default)]
struct SeverityCounts {
    critical: usize,
    high: usize,
    medium: usize,
    low: usize,
    info: usize,
}

impl SeverityCounts {
    fn from_summary(summary: &ScanSummary) -> Self {
        let mut counts = SeverityCounts::default();

        for finding in &summary.findings {
            match finding.severity {
                Severity::Critical => counts.critical += 1,
                Severity::High => counts.high += 1,
                Severity::Medium => counts.medium += 1,
                Severity::Low => counts.low += 1,
                Severity::Info => counts.info += 1,
            }
        }

        counts
    }
}
