use serde_json::Value;
use std::path::Path;

pub(super) fn resolve_tool_paths(arguments: &mut Value, root: &Path) -> Result<(), String> {
    let confinement = repopilot::path_security::RootConfinement::named(root, "MCP root")?;
    for key in ["path", "config", "baseline", "intent_path"] {
        let Some(value) = arguments.get(key).and_then(Value::as_str) else {
            continue;
        };
        let resolved = confinement.resolve_allow_missing(Path::new(value), &format!("`{key}`"))?;
        arguments[key] = Value::String(resolved.to_string_lossy().to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::resolve_tool_paths;
    use serde_json::json;
    use std::fs;

    #[test]
    fn intent_path_cannot_escape_the_configured_mcp_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("repo/subdir");
        fs::create_dir_all(&root).expect("MCP root");
        fs::write(
            temp.path().join("repo/intent.toml"),
            "version = 1\npaths = [\"src/**\"]\n",
        )
        .expect("outside intent");
        let mut arguments = json!({ "intent_path": "../intent.toml" });

        let error = resolve_tool_paths(&mut arguments, &root).expect_err("path must be denied");

        assert!(error.contains("must stay within MCP root"), "{error}");
    }
}
