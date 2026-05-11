pub mod baseline;
pub mod compare;
pub mod doctor;
pub mod harden;
pub mod init;
mod llm;
pub mod prompt;
pub mod review;
pub mod scan;
pub mod vibe;

use crate::cli::{Cli, Commands, SeverityArg};
use repopilot::config::model::RepoPilotConfig;
use repopilot::findings::types::Severity;
use repopilot::output::vibe::VibeCategory;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::types::ScanSummary;
use std::fmt;

pub const VALID_FOCUS_VALUES: &str = "security, arch, architecture, quality, framework, all";

pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Scan {
            path,
            format,
            output,
            receipt,
            config,
            baseline,
            fail_on,
            max_file_loc,
            max_directory_modules,
            max_directory_depth,
            exclude,
            include_low_signal,
            max_file_size,
            max_files,
            workspace,
            min_severity,
            verbose,
            preset,
        } => scan::run(
            path,
            format,
            output,
            receipt,
            config,
            baseline,
            fail_on,
            max_file_loc,
            max_directory_modules,
            max_directory_depth,
            exclude,
            include_low_signal,
            max_file_size,
            max_files,
            workspace,
            min_severity.map(severity_arg_into),
            verbose,
            preset,
        ),

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
            min_severity,
        } => review::run(
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
            min_severity.map(severity_arg_into),
        ),

        Commands::Baseline { command } => baseline::run(command),

        Commands::Compare {
            before,
            after,
            format,
            output,
        } => compare::run(before, after, format, output),

        Commands::Init { force, path } => init::run(force, path),

        Commands::Harden {
            path,
            config,
            focus,
            budget,
            output,
        } => harden::run(path, config, focus, budget, output),

        Commands::Prompt {
            path,
            config,
            focus,
            budget,
            output,
        } => prompt::run(path, config, focus, budget, output),

        Commands::Vibe {
            path,
            config,
            focus,
            budget,
            output,
            no_header,
        } => vibe::run(path, config, focus, budget, output, no_header),

        Commands::Doctor {
            path,
            config,
            format,
            output,
            include_low_signal,
            max_files,
        } => doctor::run(path, config, format, output, include_low_signal, max_files),
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

pub fn severity_arg_into(arg: SeverityArg) -> Severity {
    match arg {
        SeverityArg::Info => Severity::Info,
        SeverityArg::Low => Severity::Low,
        SeverityArg::Medium => Severity::Medium,
        SeverityArg::High => Severity::High,
        SeverityArg::Critical => Severity::Critical,
    }
}

pub fn parse_focus_category(
    focus: Option<&str>,
) -> Result<Option<VibeCategory>, Box<dyn std::error::Error>> {
    match focus {
        Some(value) => Ok(Some(value.parse::<VibeCategory>().map_err(|_| {
            CliExit {
                code: 2,
                message: format!("Invalid focus '{value}'. Expected: {VALID_FOCUS_VALUES}"),
            }
        })?)),
        None => Ok(None),
    }
}

#[derive(Debug, Default)]
pub struct ScanConfigOverrides {
    pub max_file_loc: Option<usize>,
    pub max_directory_modules: Option<usize>,
    pub max_directory_depth: Option<usize>,
    pub exclude_patterns: Vec<String>,
    pub include_low_signal: bool,
    pub max_file_size: Option<u64>,
    pub max_files: Option<usize>,
}

pub fn build_scan_config(
    repo_config: &RepoPilotConfig,
    overrides: ScanConfigOverrides,
) -> ScanConfig {
    let mut config = repo_config.to_scan_config();

    if let Some(threshold) = overrides.max_file_loc {
        config = config.with_large_file_loc_threshold(threshold);
    }

    if let Some(modules) = overrides.max_directory_modules {
        config.max_directory_modules = modules;
    }

    if let Some(depth) = overrides.max_directory_depth {
        config.max_directory_depth = depth;
    }

    config.exclude_patterns = overrides.exclude_patterns;
    config.include_low_signal = overrides.include_low_signal;
    if let Some(bytes) = overrides.max_file_size {
        config.max_file_bytes = bytes;
    }
    config.max_files = overrides.max_files;

    config
}

pub fn apply_min_severity_filter(summary: &mut ScanSummary, min: Severity) {
    summary.findings.retain(|finding| finding.severity >= min);
    summary.health_score =
        ScanSummary::compute_health_score(&summary.findings, summary.lines_of_code);
}
