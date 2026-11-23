//! Explain one unified review signal from a stored MCP review analysis.

use serde_json::{Value, json};

pub const TOOL_NAME: &str = "repopilot_explain_review_signal";

pub fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Explain one signal from a review: where it came from, its confidence tier, whether it can fail a gate, the files it affects, how to verify it, and its limits. The text block is a readable Markdown summary; structuredContent contains the full JSON explanation. Use it after repopilot_review_change, with a `signal_id` from that report's `tiered_signals`. For a finding (`finding_id` in the findings array), use repopilot_explain_finding instead. Reads the stored review only; it runs nothing and changes nothing.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "signal_id": {
                    "type": "string",
                    "description": "The `signal_id` of an entry in the review report's `tiered_signals`."
                },
                "path": {
                    "type": "string",
                    "description": "Optional occurrence path for a signal ID that appears more than once."
                },
                "line": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional occurrence line for a signal ID that appears more than once."
                },
                "analysis_handle": {
                    "type": "string",
                    "description": "Optional `analysisHandle` from an earlier repopilot_review_change call. Omit to use the latest review in this session."
                }
            },
            "required": ["signal_id"],
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

pub fn call(arguments: &Value, review_report: Option<&str>) -> Result<String, String> {
    let signal_id = arguments
        .get("signal_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "`signal_id` is required".to_string())?;
    let report = parse_review_report(review_report)?;
    let (occurrence_path, occurrence_line) = occurrence(arguments)?;
    let matches = render::find_signals(&report, signal_id, occurrence_path, occurrence_line);
    let signal = match matches.as_slice() {
        [] => return Err(format!("review signal not found: {signal_id}")),
        [signal] => *signal,
        _ => return ambiguous_signal(signal_id, &matches),
    };
    render::render_explanation(&report, signal_id, signal)
}

fn parse_review_report(review_report: Option<&str>) -> Result<Value, String> {
    let report = review_report.ok_or_else(|| {
        "no review is available in this MCP session; run repopilot_review_change first".to_string()
    })?;
    serde_json::from_str(report).map_err(|error| format!("review report is invalid: {error}"))
}

fn occurrence(arguments: &Value) -> Result<(Option<&str>, Option<u64>), String> {
    let occurrence_path = arguments
        .get("path")
        .map(|value| {
            value
                .as_str()
                .filter(|path| !path.is_empty())
                .ok_or_else(|| "`path` must be a non-empty string".to_string())
        })
        .transpose()?;
    let occurrence_line = arguments
        .get("line")
        .map(|value| {
            value
                .as_u64()
                .filter(|line| *line > 0)
                .ok_or_else(|| "`line` must be a positive integer".to_string())
        })
        .transpose()?;
    Ok((occurrence_path, occurrence_line))
}

fn ambiguous_signal(signal_id: &str, matches: &[&Value]) -> Result<String, String> {
    serde_json::to_string_pretty(&json!({
        "status": "ambiguous",
        "signal_id": signal_id,
        "candidates": matches.iter().map(|signal| json!({
            "path": signal.get("path").and_then(Value::as_str).unwrap_or_default(),
            "line": signal.get("line").or_else(|| signal.get("line_start"))
        })).collect::<Vec<_>>()
    }))
    .map_err(|error| format!("render ambiguous review signal: {error}"))
}

pub(super) mod render;

#[cfg(test)]
mod tests {
    use super::render::why_it_matters;
    use super::*;

    #[test]
    fn explains_signal() {
        let report = json!({
            "tiered_signals": {
                "definitely": [{
                    "signal_id": "abc123",
                    "family": "boundary",
                    "path": "src/auth.rs",
                    "headline": "access control changed",
                    "gate_eligible": true,
                    "suppressed": false,
                    "verification_plan": { "steps": ["Confirm authorization behavior."] }
                }],
                "maybe": [],
                "noise": []
            },
            "impact_paths": {
                "files": [{ "path": "src/auth.rs", "direct_dependents": ["src/api.rs"] }]
            },
            "evidence": {
                "class": "suspicion",
                "coverage_status": "limited"
            }
        })
        .to_string();

        let rendered =
            call(&json!({ "signal_id": "abc123" }), Some(&report)).expect("explain signal");
        let value: Value = serde_json::from_str(&rendered).expect("valid JSON");
        assert_eq!(value["status"], "explained");
        assert_eq!(value["gate"]["eligible"], true);
        assert_eq!(value["impact"]["direct_dependents"][0], "src/api.rs");
        assert_eq!(value["evidence"]["class"], "suspicion");
    }

    #[test]
    fn explains_rust_public_function_arity_signal() {
        let signal = json!({
            "kind": "behavioral.rust-public-function-arity-changed",
            "family": "behavioral",
            "headline": "Rust public function arity changed"
        });

        assert!(why_it_matters(&signal).contains("still passes the previous number of arguments"));
        assert!(why_it_matters(&signal).contains("does not run the Rust compiler"));
    }

    #[test]
    fn explains_rust_public_enum_variant_signal_and_repository_boundary() {
        let signal = json!({
            "kind": "behavioral.rust-public-enum-variant-removed",
            "family": "behavioral",
            "headline": "Rust public enum variant changed"
        });

        let explanation = why_it_matters(&signal);
        assert!(explanation.contains("declaration and caller locations"));
        assert!(explanation.contains("downstream crate breakage"));
    }

    #[test]
    fn duplicate_signal_id_requires_an_occurrence_locator() {
        let report = json!({
            "tiered_signals": {
                "definitely": [
                    {"signal_id": "same", "kind": "security", "path": "src/app.rs", "line": 4},
                    {"signal_id": "same", "kind": "security", "path": "src/app.rs", "line": 9}
                ],
                "maybe": [], "noise": []
            }
        })
        .to_string();

        let ambiguous =
            call(&json!({ "signal_id": "same" }), Some(&report)).expect("ambiguity response");
        let ambiguous: Value = serde_json::from_str(&ambiguous).expect("ambiguity JSON");
        assert_eq!(ambiguous["status"], "ambiguous");
        assert_eq!(
            ambiguous["candidates"]
                .as_array()
                .expect("candidates")
                .len(),
            2
        );

        let explained = call(
            &json!({ "signal_id": "same", "path": "src/app.rs", "line": 9 }),
            Some(&report),
        )
        .expect("selected occurrence");
        let explained: Value = serde_json::from_str(&explained).expect("explanation JSON");
        assert_eq!(explained["status"], "explained");
        assert_eq!(explained["signal"]["line"], 9);
    }
}
