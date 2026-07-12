//! Explain one unified review signal from a stored MCP review analysis.

use serde_json::{Value, json};

pub const TOOL_NAME: &str = "repopilot_explain_review_signal";

pub fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Explain one review signal by signal_id from the latest or a handle-selected review. Returns provenance, trust tier, gate eligibility, impact context, verification steps, and limitations.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "signal_id": { "type": "string" },
                "analysis_handle": { "type": "string" }
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

    let result = json!({
        "status": "explained",
        "signal_id": signal_id,
        "signal": signal,
        "why_it_matters": why_it_matters(signal),
        "impact": impact_for_path(&report, path),
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
            }
        })
        .to_string();

        let rendered =
            call(&json!({ "signal_id": "abc123" }), Some(&report)).expect("explain signal");
        let value: Value = serde_json::from_str(&rendered).expect("valid JSON");
        assert_eq!(value["status"], "explained");
        assert_eq!(value["gate"]["eligible"], true);
        assert_eq!(value["impact"]["direct_dependents"][0], "src/api.rs");
    }
}
