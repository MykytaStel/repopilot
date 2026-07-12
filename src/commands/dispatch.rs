use crate::cli::{AiCommands, Cli, Commands};
use std::fmt;

pub const EXIT_FINDINGS: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_RUNTIME: i32 = 3;

pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Cache(options) => super::cache::run(options.command),
        Commands::Scan(options) => super::scan::run(options),
        Commands::Review(options) => super::review::run(options),
        Commands::Snapshot(options) => super::snapshot::run(options),
        Commands::Baseline(options) => super::baseline::run(options.command),
        Commands::Ai(options) => run_ai(options.command),
        Commands::Init(options) => super::init::run(options),
        Commands::Mcp(options) => super::mcp::run(options),
    }
}

fn run_ai(command: AiCommands) -> Result<(), Box<dyn std::error::Error>> {
    let AiCommands::Context(options) = command;
    super::ai_context::run(options)
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
