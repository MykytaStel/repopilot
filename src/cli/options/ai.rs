use crate::cli::parse_token_budget;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct AiOptions {
    #[command(subcommand)]
    pub command: AiCommands,
}

#[derive(Subcommand)]
pub enum AiCommands {
    /// Generate an LLM-ready handoff (context, prioritized plan, and guidance)
    #[command(
        about = "Generate an LLM-ready handoff: repository context, evidence, and a prioritized fix plan",
        after_help = "EXAMPLES:\n  \
repopilot ai context .\n  \
repopilot ai context . --focus security --budget 8k\n  \
repopilot ai context . --no-task --output ai-context.md"
    )]
    Context(AiContextOptions),
}

#[derive(Args)]
pub struct AiContextOptions {
    /// Path to project, folder, or file to scan
    pub path: PathBuf,

    /// Path to a repopilot.toml config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Limit output to one finding category
    #[arg(long, value_parser = ["security", "arch", "architecture", "quality", "framework", "all"])]
    pub focus: Option<String>,

    /// Target token budget: 2k, 4k, 8k, 16k, or a positive integer
    #[arg(long, value_parser = parse_token_budget)]
    pub budget: Option<usize>,

    /// Write output to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Output format for the handoff: markdown (default, human-readable) or json (structured, for agents)
    #[arg(long, value_parser = ["markdown", "json"], default_value = "markdown")]
    pub format: String,

    /// Omit the intro header block (Markdown output only)
    #[arg(long)]
    pub no_header: bool,

    /// Omit the AI task instruction block — the "> Instructions for AI assistant:" preamble (Markdown output only; JSON is always fact-only)
    #[arg(long)]
    pub no_task: bool,

    /// Print a per-section token breakdown to stderr (enabled automatically when stdout is a TTY)
    #[arg(long)]
    pub show_breakdown: bool,
}
