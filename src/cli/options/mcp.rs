use clap::Args;
use std::path::PathBuf;

/// Options for `repopilot mcp`.
///
#[derive(Args)]
pub struct McpOptions {
    /// Restrict all MCP file access to this workspace root
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// Maximum serialized size of one MCP tool result
    #[arg(long, default_value_t = 1_048_576, value_parser = parse_response_limit)]
    pub max_response_bytes: usize,
}

fn parse_response_limit(raw: &str) -> Result<usize, String> {
    let value = raw
        .parse::<usize>()
        .map_err(|_| "response limit must be a positive integer".to_string())?;
    if value < 1024 {
        return Err("response limit must be at least 1024 bytes".to_string());
    }
    Ok(value)
}
