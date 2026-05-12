use crate::cli::BaselineCommands;
use chrono::{SecondsFormat, Utc};
use repopilot::baseline::key::normalized_relative_path;
use repopilot::baseline::model::Baseline;
use repopilot::baseline::writer::{BaselineWriteError, write_baseline};
use repopilot::config::loader::load_default_config;
use repopilot::findings::types::Severity;
use repopilot::scan::scanner::scan_path_with_config;
use repopilot::scan::types::ScanSummary;
use std::path::{Path, PathBuf};

pub fn run(command: BaselineCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BaselineCommands::Create(options) => {
            let path = options.path;
            let output = options.output;
            let force = options.force;
            let output_path = output.unwrap_or_else(|| default_baseline_path(&path));

            if output_path.exists() && !force {
                return Err(format!(
                    "Baseline already exists at {}. Use `repopilot baseline create {} --force` to overwrite it.",
                    output_path.display(),
                    path.display()
                )
                .into());
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
                Err(BaselineWriteError::AlreadyExists { .. }) => Err(format!(
                    "Baseline already exists at {}. Use `repopilot baseline create {} --force` to overwrite it.",
                    output_path.display(),
                    path.display()
                )
                .into()),
                Err(error) => Err(Box::new(error)),
            }
        }
    }
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
