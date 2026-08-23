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
