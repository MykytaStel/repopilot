use clap::Args;

/// Options for `repopilot mcp`.
///
/// The server speaks the Model Context Protocol over stdio, so it takes no
/// flags today: an MCP client launches it and drives it with JSON-RPC requests.
/// Per-call inputs (such as the repository path) are tool arguments, not CLI
/// flags. The struct exists so the subcommand can grow options without a
/// breaking change.
#[derive(Args)]
pub struct McpOptions {}
