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
    /// Generate LLM-ready repository context from a scan
    #[command(
        about = "Generate LLM-ready repository context from a scan",
        after_help = "EXAMPLES:\n  \
repopilot ai context .\n  \
repopilot ai context . --focus security --budget 2k\n  \
repopilot ai context . --no-header | pbcopy"
    )]
    Context(AiContextOptions),

    /// Generate a prioritized remediation plan from scan findings
    #[command(
        about = "Generate a prioritized remediation plan from scan findings",
        after_help = "EXAMPLES:\n  \
repopilot ai plan .\n  \
repopilot ai plan . --focus security --budget 4k\n  \
repopilot ai plan . --output ai-plan.md"
    )]
    Plan(AiPlanOptions),

    /// Generate an AI-ready remediation prompt from scan findings
    #[command(
        about = "Generate an AI-ready remediation prompt from scan findings",
        after_help = "EXAMPLES:\n  \
repopilot ai prompt .\n  \
repopilot ai prompt . --focus quality --budget 4k\n  \
repopilot ai prompt . --output prompt.md"
    )]
    Prompt(AiPromptOptions),
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

    /// Write Markdown output to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Omit the intro header block
    #[arg(long)]
    pub no_header: bool,

    /// Omit the AI task instruction block (the "> Instructions for AI assistant:" preamble)
    #[arg(long)]
    pub no_task: bool,

    /// Print a per-section token breakdown to stderr (enabled automatically when stdout is a TTY)
    #[arg(long)]
    pub show_breakdown: bool,
}

#[derive(Args)]
pub struct AiPlanOptions {
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

    /// Write Markdown output to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct AiPromptOptions {
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

    /// Write Markdown output to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}
