pub mod baseline;
pub mod compare;
pub mod doctor;
pub mod explain;
pub mod harden;
pub mod init;
pub mod knowledge;
mod llm;
mod progress;
pub mod prompt;
pub mod review;
pub mod scan;
pub mod vibe;

use crate::cli::{
    AiCommands, Cli, Commands, InspectCommands, PriorityArg, ReviewOptions, ScanOptions,
    SeverityArg,
};
use repopilot::config::model::RepoPilotConfig;
use repopilot::findings::types::{Finding, Severity};
use repopilot::risk::RiskPriority;
use repopilot::output::vibe::VibeCategory;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::types::ScanSummary;
use std::fmt;

pub const VALID_FOCUS_VALUES: &str = "security, arch, architecture, quality, framework, all";
pub const EXIT_FINDINGS: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_RUNTIME: i32 = 3;

pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Scan(options) => scan::run(options),
        Commands::Review(options) => review::run(options),
        Commands::Baseline(options) => baseline::run(options.command),
        Commands::Compare(options) => compare::run(
            options.before,
            options.after,
            options.format,
            options.output,
        ),
        Commands::Ai(options) => run_ai(options.command),
        Commands::Inspect(options) => run_inspect(options.command),
        Commands::Init(options) => init::run(options.force, options.path),
        Commands::Doctor(options) => doctor::run(
            options.path,
            options.config,
            options.format,
            options.output,
            options.include_low_signal,
            options.max_files,
        ),
        Commands::Vibe(options) => vibe::run(options),
        Commands::Harden(options) => harden::run(
            options.path,
            options.config,
            options.focus,
            options.budget,
            options.output,
        ),
        Commands::Prompt(options) => prompt::run(
            options.path,
            options.config,
            options.focus,
            options.budget,
            options.output,
        ),
        Commands::Explain(options) => explain::run(
            options.path,
            options.rule,
            options.signal,
            options.severity,
            options.format,
            options.output,
        ),
        Commands::Knowledge(options) => {
            knowledge::run(options.section, options.format, options.output)
        }
    }
}

fn run_ai(command: AiCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        AiCommands::Context(options) => vibe::run(options),
        AiCommands::Plan(options) => harden::run(
            options.path,
            options.config,
            options.focus,
            options.budget,
            options.output,
        ),
        AiCommands::Prompt(options) => prompt::run(
            options.path,
            options.config,
            options.focus,
            options.budget,
            options.output,
        ),
    }
}

fn run_inspect(command: InspectCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        InspectCommands::Explain(options) => explain::run(
            options.path,
            options.rule,
            options.signal,
            options.severity,
            options.format,
            options.output,
        ),
        InspectCommands::Knowledge(options) => {
            knowledge::run(options.section, options.format, options.output)
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

pub fn severity_arg_into(arg: SeverityArg) -> Severity {
    match arg {
        SeverityArg::Info => Severity::Info,
        SeverityArg::Low => Severity::Low,
        SeverityArg::Medium => Severity::Medium,
        SeverityArg::High => Severity::High,
        SeverityArg::Critical => Severity::Critical,
    }
}

pub fn priority_arg_into(arg: PriorityArg) -> RiskPriority {
    arg.into()
}

pub fn parse_focus_category(
    focus: Option<&str>,
) -> Result<Option<VibeCategory>, Box<dyn std::error::Error>> {
    match focus {
        Some(value) => Ok(Some(value.parse::<VibeCategory>().map_err(|_| {
            CliExit {
                code: EXIT_USAGE,
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

pub fn apply_min_priority_filter(summary: &mut ScanSummary, min: RiskPriority) {
    summary
        .findings
        .retain(|finding| finding_meets_min_priority(finding, min));
    summary.health_score =
        ScanSummary::compute_health_score(&summary.findings, summary.lines_of_code);
}

pub fn finding_meets_min_priority(finding: &Finding, min: RiskPriority) -> bool {
    priority_rank(finding.risk.priority) <= priority_rank(min)
}

fn priority_rank(priority: RiskPriority) -> u8 {
    match priority {
        RiskPriority::P0 => 0,
        RiskPriority::P1 => 1,
        RiskPriority::P2 => 2,
        RiskPriority::P3 => 3,
    }
}

pub fn scan_options_min_severity(options: &ScanOptions) -> Option<Severity> {
    options.min_severity.map(severity_arg_into)
}

pub fn scan_options_min_priority(options: &ScanOptions) -> Option<RiskPriority> {
    options.min_priority.map(priority_arg_into)
}

pub fn review_options_min_severity(options: &ReviewOptions) -> Option<Severity> {
    options.min_severity.map(severity_arg_into)
}

pub fn review_options_min_priority(options: &ReviewOptions) -> Option<RiskPriority> {
    options.min_priority.map(priority_arg_into)
}
