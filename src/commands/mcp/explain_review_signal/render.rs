use serde_json::{Value, json};

pub(super) fn render_explanation(
    report: &Value,
    signal_id: &str,
    signal: &Value,
) -> Result<String, String> {
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
        "impact": impact_for_path(report, impact_path),
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

pub(super) fn find_signals<'a>(
    report: &'a Value,
    signal_id: &str,
    path: Option<&str>,
    line: Option<u64>,
) -> Vec<&'a Value> {
    let Some(tiered) = report.get("tiered_signals") else {
        return Vec::new();
    };
    ["definitely", "maybe", "noise"]
        .into_iter()
        .filter_map(|tier| tiered.get(tier).and_then(Value::as_array))
        .flatten()
        .filter(|signal| signal.get("signal_id").and_then(Value::as_str) == Some(signal_id))
        .filter(|signal| {
            path.is_none_or(|path| signal.get("path").and_then(Value::as_str) == Some(path))
        })
        .filter(|signal| {
            line.is_none_or(|line| {
                signal
                    .get("line")
                    .or_else(|| signal.get("line_start"))
                    .and_then(Value::as_u64)
                    == Some(line)
            })
        })
        .collect()
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

pub(super) fn why_it_matters(signal: &Value) -> String {
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
    if signal.get("kind").and_then(Value::as_str)
        == Some("behavioral.rust-public-struct-field-removed")
    {
        return "a public Rust struct field was removed or made private while a proven direct local struct literal still names it. This is static Git-diff evidence; RepoPilot does not run the Rust compiler.".to_string();
    }
    if signal.get("kind").and_then(Value::as_str)
        == Some("behavioral.rust-public-enum-variant-removed")
    {
        return "a public Rust enum variant was removed while a proven local constructor or match still names it. The finding retains declaration and caller locations; RepoPilot does not run the Rust compiler or claim downstream crate breakage.".to_string();
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
