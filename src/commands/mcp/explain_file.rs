//! The `repopilot_explain_file` MCP tool: how RepoPilot classifies one file and
//! which rules/signals apply, wrapping the `repopilot::explain` report builder.

use repopilot::explain::{
    base_severity_for_explain, build_explain_report_with_root, render_explain_report,
};
use repopilot::output::OutputFormat;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub const TOOL_NAME: &str = "repopilot_explain_file";

pub fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Explain one file with role evidence, applicability checks, ordered knowledge overrides, severity transitions, default-profile visibility, and explicit scope limits. Returns a JSON explanation. Local-only.",
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
        },
        "outputSchema": { "type": "object", "additionalProperties": true },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        }
    })
}

pub fn call(arguments: &Value, root: &Path) -> Result<String, String> {
    let Some(path) = arguments.get("path").and_then(Value::as_str) else {
        return Err("`path` is required".to_string());
    };
    let path = PathBuf::from(path);
    let rule = arguments.get("rule").and_then(Value::as_str);
    let signal = arguments.get("signal").and_then(Value::as_str);

    let base_severity = base_severity_for_explain(rule, signal);
    let report = build_explain_report_with_root(root, &path, rule, signal, base_severity)
        .map_err(|error| format!("explain failed: {error}"))?;

    render_explain_report(&report, OutputFormat::Json)
        .map_err(|error| format!("render failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explain_tool_returns_role_evidence_scope_and_decision_trace() {
        let rendered = call(
            &json!({
                "path": "src/lib.rs",
                "rule": "language.rust.panic-risk",
                "signal": "rust.panic"
            }),
            Path::new("."),
        )
        .expect("explain tool call");
        let value: Value = serde_json::from_str(&rendered).expect("explain JSON");
        assert_eq!(value["scope"]["analysis_scope"], "single-file");
        assert!(value["context"]["role_evidence"].is_array());
        assert!(value["decision"]["trace"].is_array());
        assert!(
            value["decision"]["trace"]
                .as_array()
                .is_some_and(|steps| !steps.is_empty())
        );
    }
}
