//! The `repopilot_explain_file` MCP tool: how RepoPilot classifies one file and
//! which rules/signals apply, wrapping the `inspect explain` builder.

use repopilot::explain::{build_explain_report, render_explain_report};
use repopilot::findings::types::Severity;
use repopilot::output::OutputFormat;
use serde_json::{Value, json};
use std::path::PathBuf;

pub const TOOL_NAME: &str = "repopilot_explain_file";

pub fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Explain how RepoPilot classifies a single file (role, language, test/production context) and which rules and signals apply, with the resulting severity decisions. Returns a JSON explanation. Local-only.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to explain."
                },
                "rule": {
                    "type": "string",
                    "description": "Optional rule id to focus the explanation on."
                },
                "signal": {
                    "type": "string",
                    "description": "Optional signal id to focus the explanation on."
                }
            },
            "required": ["path"],
            "additionalProperties": false
        }
    })
}

pub fn call(arguments: &Value) -> Result<String, String> {
    let Some(path) = arguments.get("path").and_then(Value::as_str) else {
        return Err("`path` is required".to_string());
    };
    let path = PathBuf::from(path);
    let rule = arguments.get("rule").and_then(Value::as_str);
    let signal = arguments.get("signal").and_then(Value::as_str);

    // Lowest base severity so the explanation surfaces every applicable rule.
    let report = build_explain_report(&path, rule, signal, Severity::Info)
        .map_err(|error| format!("explain failed: {error}"))?;

    render_explain_report(&report, OutputFormat::Json)
        .map_err(|error| format!("render failed: {error}"))
}
