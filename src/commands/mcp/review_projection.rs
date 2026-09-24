use serde_json::Value;

pub(super) fn compact_review_json(rendered: &str) -> Result<String, String> {
    const LIMIT: usize = 20;
    let mut value: Value =
        serde_json::from_str(rendered).map_err(|error| format!("compact failed: {error}"))?;
    if let Some(findings) = value.get_mut("findings").and_then(Value::as_array_mut) {
        findings.truncate(LIMIT);
    }
    let mut remaining = LIMIT;
    for tier in ["definitely", "maybe", "noise"] {
        if let Some(signals) = value
            .get_mut("tiered_signals")
            .and_then(|tiered| tiered.get_mut(tier))
            .and_then(Value::as_array_mut)
        {
            signals.truncate(remaining);
            remaining = remaining.saturating_sub(signals.len());
        }
    }
    serde_json::to_string_pretty(&value).map_err(|error| format!("compact failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::compact_review_json;

    #[test]
    fn compact_projection_preserves_verification_diagnostics() {
        let rendered = r#"{
            "merge_readiness": {
                "verification": [{
                    "check_id": "python.tests",
                    "diagnostics": {
                        "adapter": "pytest-node-v1",
                        "complete": true,
                        "entries": [{"kind": "failed-test-node", "key": "python.tests:tests/test_api.py::test_create:failed"}]
                    }
                }]
            },
            "findings": [],
            "tiered_signals": {"definitely": [], "maybe": [], "noise": []}
        }"#;

        let compacted = compact_review_json(rendered).expect("compact review");
        assert!(compacted.contains("pytest-node-v1"));
        assert!(compacted.contains("python.tests:tests/test_api.py::test_create:failed"));
    }
}
