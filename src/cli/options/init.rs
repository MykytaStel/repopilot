use clap::{Args, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum McpClientArg {
    Claude,
    Cursor,
    Generic,
}

#[derive(Args)]
pub struct InitOptions {
    /// Overwrite RepoPilot-owned generated files
    #[arg(long)]
    pub force: bool,

    /// Config path; defaults to repopilot.toml at the Git root or current directory
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// Generate a review-first GitHub Actions workflow
    #[arg(long)]
    pub github_action: bool,

    /// Generate an MCP client configuration example
    #[arg(long, value_enum)]
    pub mcp_client: Option<McpClientArg>,

    /// Generate config, GitHub Action, and generic MCP bootstrap files
    #[arg(long)]
    pub all: bool,
}
