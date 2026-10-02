//! MCP bootstrap files written by `repopilot init --mcp-client`: one thin
//! adapter per agent client over the same `repopilot mcp --root .` command.

use crate::cli::McpClientArg;
use std::path::{Path, PathBuf};

pub(super) const MCP_DIR: &str = ".repopilot/bootstrap";

pub(super) fn mcp_output_path(client: McpClientArg) -> PathBuf {
    let name = match client {
        McpClientArg::Claude => "claude.json",
        McpClientArg::Codex => "codex.json",
        McpClientArg::Copilot => "copilot.json",
        McpClientArg::Cursor => "cursor.json",
        McpClientArg::Gemini => "gemini.json",
        McpClientArg::Generic => "generic.json",
    };
    Path::new(MCP_DIR).join(name)
}

pub(super) fn mcp_bootstrap(client: McpClientArg) -> String {
    match client {
        McpClientArg::Claude => r#"{
  "registration_command": "claude mcp add repopilot -- repopilot mcp --root .",
  "note": "Run the registration command from the repository root."
}
"#
        .to_string(),
        McpClientArg::Codex => r#"{
  "registration_command": "codex mcp add repopilot -- repopilot mcp --root .",
  "note": "Run the registration command from the repository root. The RepoPilot Codex plugin also registers this server."
}
"#
        .to_string(),
        McpClientArg::Copilot => r#"{
  "mcpServers": {
    "repopilot": {
      "type": "local",
      "command": "repopilot",
      "args": ["mcp", "--root", "."],
      "tools": ["*"]
    }
  },
  "note": "Paste into the repository's Copilot coding agent MCP configuration, and install the repopilot CLI in .github/workflows/copilot-setup-steps.yml."
}
"#
        .to_string(),
        McpClientArg::Gemini => r#"{
  "mcpServers": {
    "repopilot": {
      "command": "repopilot",
      "args": ["mcp", "--root", "."]
    }
  },
  "note": "Merge the mcpServers entry into .gemini/settings.json."
}
"#
        .to_string(),
        McpClientArg::Cursor => r#"{
  "mcpServers": {
    "repopilot": {
      "command": "repopilot",
      "args": ["mcp", "--root", "."]
    }
  },
  "note": "Copy this server entry into the MCP configuration used by Cursor."
}
"#
        .to_string(),
        McpClientArg::Generic => r#"{
  "mcpServers": {
    "repopilot": {
      "command": "repopilot",
      "args": ["mcp", "--root", "."]
    }
  }
}
"#
        .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mcp_bootstrap_is_valid_json_and_launches_or_registers_repopilot() {
        for client in [
            McpClientArg::Claude,
            McpClientArg::Codex,
            McpClientArg::Copilot,
            McpClientArg::Cursor,
            McpClientArg::Gemini,
            McpClientArg::Generic,
        ] {
            let value: serde_json::Value =
                serde_json::from_str(&mcp_bootstrap(client)).expect("valid bootstrap JSON");
            let rendered = value.to_string();
            assert!(rendered.contains("repopilot"));
            assert!(rendered.contains("mcp"));
        }
    }
}
