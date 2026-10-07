use serde_json::Value;
use std::path::Path;

pub(super) fn resolve_tool_paths(arguments: &mut Value, root: &Path) -> Result<(), String> {
    let confinement = repopilot::path_security::RootConfinement::named(root, "MCP root")?;
    for key in ["path", "config", "baseline"] {
        let Some(value) = arguments.get(key).and_then(Value::as_str) else {
            continue;
        };
        let resolved = confinement.resolve_allow_missing(Path::new(value), &format!("`{key}`"))?;
        arguments[key] = Value::String(resolved.to_string_lossy().to_string());
    }
    Ok(())
}
