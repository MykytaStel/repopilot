pub mod baseline;
pub mod cache;
pub mod cache_inspect;
pub mod compare;
pub mod doctor;
pub mod explain;
pub(crate) mod filters;
pub(crate) mod focus;
pub mod harden;
pub mod init;
pub mod knowledge;
mod llm;
mod progress;
pub mod prompt;
pub mod review;
pub mod scan;
pub(crate) mod scan_config;
pub mod vibe;

use crate::cli::{AiCommands, Cli, Commands, InspectCommands};
use std::fmt;

pub const EXIT_FINDINGS: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_RUNTIME: i32 = 3;

pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Cache(options) => cache::run(options.command),
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
        InspectCommands::Cache(options) => {
            cache_inspect::run(options.path, options.format, options.output)
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
