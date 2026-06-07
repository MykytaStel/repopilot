use clap::Args;
use std::path::PathBuf;

/// Options for `repopilot mcp`.
///
#[derive(Args)]
pub struct McpOptions {
    /// Restrict all MCP file access to this workspace root
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
}
