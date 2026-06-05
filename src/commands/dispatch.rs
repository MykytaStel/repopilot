use crate::cli::{AiCommands, Cli, Commands, InspectCommands};
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
        Commands::Compare(options) => super::compare::run(
            options.before,
            options.after,
            options.format,
            options.output,
        ),
        Commands::Ai(options) => run_ai(options.command),
        Commands::Inspect(options) => run_inspect(options.command),
        Commands::Init(options) => super::init::run(options.force, options.path),
        Commands::Doctor(options) => super::doctor::run(
            options.path,
            options.config,
            options.format,
            options.output,
            options.include_low_signal,
            options.max_files,
        ),
        Commands::Mcp(options) => super::mcp::run(options),
    }
}

fn run_ai(command: AiCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        AiCommands::Context(options) => super::ai_context::run(options),
        AiCommands::Plan(options) => super::ai_plan::run(
            options.path,
            options.config,
            options.focus,
            options.budget,
            options.output,
        ),
        AiCommands::Prompt(options) => super::prompt::run(
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
        InspectCommands::Explain(options) => super::explain::run(
            options.path,
            options.rule,
            options.signal,
            options.severity,
            options.format,
            options.output,
        ),
        InspectCommands::Knowledge(options) => {
            super::knowledge::run(options.section, options.format, options.output)
        }
        InspectCommands::Cache(options) => {
            super::cache_inspect::run(options.path, options.format, options.output)
        }
        InspectCommands::Graph(options) => {
            super::graph::run(options.path, options.config, options.format, options.output)
        }
        InspectCommands::Feedback(options) => super::feedback::run(
            options.path,
            options.format,
            options.output,
            options.evaluate,
        ),
        InspectCommands::Rules(options) => super::rules::list_rules(
            options.format,
            options.lifecycle,
            options.source,
            options.output,
        ),
        InspectCommands::Rule(options) => {
            super::rules::inspect_rule(options.rule_id, options.format, options.output)
        }
        InspectCommands::EvalRules(options) => super::rules::eval_rules(
            options.rule,
            options.fixtures,
            options.format,
            options.output,
        ),
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
