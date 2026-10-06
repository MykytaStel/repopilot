//! Explain one unified review signal from a stored MCP review analysis.

use serde_json::{Value, json};

pub const TOOL_NAME: &str = "repopilot_explain_review_signal";

pub fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Explain one signal from a review: where it came from, its confidence tier, whether it can fail a gate, the files it affects, how to verify it, and its limits. Use it after repopilot_review_change, with a `signal_id` from that report's `tiered_signals`. For a finding (`finding_id` in the findings array), use repopilot_explain_finding instead. Reads the stored review only; it runs nothing and changes nothing.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "signal_id": {
                    "type": "string",
                    "description": "The `signal_id` of an entry in the review report's `tiered_signals`."
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
    let report = review_report.ok_or_else(|| {
        "no review is available in this MCP session; run repopilot_review_change first".to_string()
    })?;
    let report: Value = serde_json::from_str(report)
        .map_err(|error| format!("review report is invalid: {error}"))?;
    let signal = find_signal(&report, signal_id)
        .ok_or_else(|| format!("review signal not found: {signal_id}"))?;
    let path = signal
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let impact_path = signal
        .get("target_path")
        .and_then(Value::as_str)
        .unwrap_or(path);

    let result = json!({
        "status": "explained",
        "signal_id": signal_id,
        "signal": signal,
        "change_proof": report.get("change_proof").cloned().unwrap_or(Value::Null),
        "evidence": report.get("evidence").cloned().unwrap_or(Value::Null),
        "decision": report.get("decision").cloned().unwrap_or(Value::Null),
        "why_it_matters": why_it_matters(signal),
        "impact": impact_for_path(&report, impact_path),
        "gate": {
            "eligible": signal.get("gate_eligible").and_then(Value::as_bool).unwrap_or(false),
            "suppressed": signal.get("suppressed").and_then(Value::as_bool).unwrap_or(false),
            "suppression_reason": signal.get("suppression_reason").cloned().unwrap_or(Value::Null)
        },
        "verification_plan": signal.get("verification_plan").cloned().unwrap_or(Value::Null),
        "limitations": [
            "Derived from stored Git diff and static review evidence.",
            "Does not execute code, run tests, observe runtime behavior, or prove exploitability.",
            "Re-run repopilot_review_change after workspace edits."
        ]
    });

    serde_json::to_string_pretty(&result)
        .map_err(|error| format!("render review-signal explanation failed: {error}"))
}

fn find_signal<'a>(report: &'a Value, signal_id: &str) -> Option<&'a Value> {
    let tiered = report.get("tiered_signals")?;
    ["definitely", "maybe", "noise"]
        .into_iter()
        .filter_map(|tier| tiered.get(tier).and_then(Value::as_array))
        .flatten()
        .find(|signal| signal.get("signal_id").and_then(Value::as_str) == Some(signal_id))
}

fn impact_for_path(report: &Value, path: &str) -> Value {
    if path.is_empty() {
        return Value::Null;
    }
    report
        .get("impact_paths")
        .and_then(|value| value.get("files"))
        .and_then(Value::as_array)
        .and_then(|files| {
            files
                .iter()
                .find(|entry| entry.get("path").and_then(Value::as_str) == Some(path))
        })
        .cloned()
        .unwrap_or_else(|| json!({ "path": path }))
}

fn why_it_matters(signal: &Value) -> String {
    if signal.get("kind").and_then(Value::as_str)
        == Some("behavioral.removed-export-still-imported")
    {
        return "removed export is still imported. The caller imports a named symbol that the changed module no longer exports, which can break that import contract. This is static Git-diff evidence; RepoPilot does not execute the compiler or claim full module-resolution parity.".to_string();
    }
    if signal.get("kind").and_then(Value::as_str)
        == Some("behavioral.rust-public-function-arity-changed")
    {
        return "a Rust public function changed its parameter count while a proven direct local call still passes the previous number of arguments. This is static Git-diff evidence; RepoPilot does not run the Rust compiler.".to_string();
    }
    let family = signal
        .get("family")
        .and_then(Value::as_str)
        .unwrap_or("review");
    let headline = signal
        .get("headline")
        .and_then(Value::as_str)
        .unwrap_or("change signal detected");
    match family {
        "boundary" => format!(
            "{headline}. Boundary changes can alter access, trust, deployment, dependency, or secret-handling behavior."
        ),
        "behavioral" => format!(
            "{headline}. Behavioral changes may affect external calls, persistence, process execution, migrations, or removed safeguards."
        ),
        "algorithmic" => format!(
            "{headline}. Algorithmic changes can affect complexity, termination, resource use, or edge-case behavior."
        ),
        "taint" => format!(
            "{headline}. This is static reachability evidence to a sensitive sink, not proof of a vulnerability."
        ),
        _ => format!("{headline}. Review the changed surface and verify intended behavior."),
    }
}

#[cfg(test)]
mod tests {
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
}
