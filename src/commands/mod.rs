pub mod baseline;
pub mod compare;
pub mod init;
pub mod review;
pub mod scan;
pub mod vibe;

use crate::cli::{Cli, Commands, SeverityArg};
use repopilot::config::model::RepoPilotConfig;
use repopilot::findings::types::Severity;
use repopilot::scan::config::ScanConfig;
use std::fmt;

pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
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
            workspace,
            min_severity,
            verbose,
            preset,
        } => scan::run(
            path,
            format,
            output,
            config,
            baseline,
            fail_on,
            max_file_loc,
            max_directory_modules,
            max_directory_depth,
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

        Commands::Vibe {
            path,
            config,
            focus,
            budget,
            output,
            no_header,
        } => vibe::run(path, config, focus, budget, output, no_header),
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

pub fn build_scan_config(
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
