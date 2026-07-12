use crate::cli::{InitOptions, McpClientArg};
use repopilot::config::template::default_config_toml;
use std::fs;
use std::path::{Path, PathBuf};

const ACTION_PATH: &str = ".github/workflows/repopilot-review.yml";
const MCP_DIR: &str = ".repopilot/bootstrap";

pub fn run(options: InitOptions) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::current_dir()?;
    run_at(options, &root)
}

fn run_at(options: InitOptions, root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let generate_action = options.github_action || options.all;
    let mcp_client = options
        .mcp_client
        .or(options.all.then_some(McpClientArg::Generic));

    let config = default_config_toml();
    write_owned_file(&options.path, &config, options.force, "RepoPilot config")?;

    if generate_action {
        write_owned_file(
            &root.join(ACTION_PATH),
            &github_action_workflow(),
            options.force,
            "GitHub Actions workflow",
        )?;
    }

    if let Some(client) = mcp_client {
        let path = root.join(mcp_output_path(client));
        write_owned_file(
            &path,
            &mcp_bootstrap(client),
            options.force,
            "MCP bootstrap",
        )?;
    }

    print_next_steps(&options.path, generate_action, mcp_client);
    Ok(())
}

fn write_owned_file(
    path: &Path,
    content: &str,
    force: bool,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() && !force {
        println!(
            "{label} already exists at {}. Use `repopilot init --force` to overwrite RepoPilot-owned generated files.",
            path.display()
        );
        return Ok(());
    }

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    println!("Created {label} at {}", path.display());
    Ok(())
}

fn github_action_workflow() -> String {
    format!(
        r#"name: RepoPilot review

on:
  pull_request:

permissions:
  contents: read
  pull-requests: write
  security-events: write

jobs:
  repopilot:
    uses: MykytaStel/repopilot/.github/workflows/repopilot-pr-review.yml@v{}
    with:
      fail-on-review: none
      upload-sarif: true
"#,
        env!("CARGO_PKG_VERSION")
    )
}

fn mcp_output_path(client: McpClientArg) -> PathBuf {
    let name = match client {
        McpClientArg::Claude => "claude.json",
        McpClientArg::Cursor => "cursor.json",
        McpClientArg::Generic => "generic.json",
    };
    Path::new(MCP_DIR).join(name)
}

fn mcp_bootstrap(client: McpClientArg) -> String {
    match client {
        McpClientArg::Claude => r#"{
  "registration_command": "claude mcp add repopilot -- repopilot mcp --root .",
  "note": "Run the registration command from the repository root."
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

fn print_next_steps(config: &Path, action: bool, mcp: Option<McpClientArg>) {
    println!();
    println!("Next:");
    println!("  repopilot review .");
    println!("  repopilot scan . --config {}", config.display());

    if action {
        println!("  git add {ACTION_PATH}");
        println!("  open a pull request to verify the generated RepoPilot workflow");
    }
    if let Some(client) = mcp {
        println!("  review {}", mcp_output_path(client).display());
        println!("  copy or register the MCP entry in your client explicitly");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn options(root: &Path) -> InitOptions {
        InitOptions {
            force: false,
            path: root.join("repopilot.toml"),
            github_action: false,
            mcp_client: None,
            all: false,
        }
    }

    #[test]
    fn init_generates_config_action_and_generic_mcp_bootstrap() {
        let temp = tempdir().expect("temp dir");
        let mut options = options(temp.path());
        options.github_action = true;
        options.mcp_client = Some(McpClientArg::Generic);

        run_at(options, temp.path()).expect("init succeeds");

        assert!(temp.path().join("repopilot.toml").is_file());
        let action = fs::read_to_string(temp.path().join(ACTION_PATH)).expect("action");
        assert!(action.contains(&format!(
            "repopilot-pr-review.yml@v{}",
            env!("CARGO_PKG_VERSION")
        )));
        assert!(action.contains("upload-sarif: true"));

        let mcp = fs::read_to_string(temp.path().join(MCP_DIR).join("generic.json")).expect("mcp");
        assert!(mcp.contains("\"repopilot\""));
        assert!(mcp.contains("\"--root\""));
    }

    #[test]
    fn init_preserves_existing_generated_files_without_force() {
        let temp = tempdir().expect("temp dir");
        let path = temp.path().join("repopilot.toml");
        fs::write(&path, "custom = true\n").expect("existing config");

        run_at(options(temp.path()), temp.path()).expect("init succeeds");

        assert_eq!(fs::read_to_string(path).expect("config"), "custom = true\n");
    }

    #[test]
    fn force_replaces_existing_config() {
        let temp = tempdir().expect("temp dir");
        let path = temp.path().join("repopilot.toml");
        fs::write(&path, "custom = true\n").expect("existing config");
        let mut options = options(temp.path());
        options.force = true;

        run_at(options, temp.path()).expect("init succeeds");

        assert_ne!(fs::read_to_string(path).expect("config"), "custom = true\n");
    }

    #[test]
    fn every_mcp_bootstrap_is_valid_json_and_launches_or_registers_repopilot() {
        for client in [
            McpClientArg::Claude,
            McpClientArg::Cursor,
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
