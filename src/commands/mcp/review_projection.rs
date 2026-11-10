use serde_json::{Value, json};

const COMPACT_LIMIT: usize = 20;
const DEFAULT_PAGE_LIMIT: usize = 100;
const MAX_PAGE_LIMIT: usize = 1_000;

pub(super) fn paginate_review_json(
    rendered: &str,
    arguments: &Value,
) -> Result<super::analysis_store::PaginatedReport, String> {
    let mut value: Value =
        serde_json::from_str(rendered).map_err(|error| format!("paginate failed: {error}"))?;
    let compact = arguments.get("detail").and_then(Value::as_str) != Some("full");
    let findings = page_selection(arguments, "offset", "limit", compact)?;
    let signals = page_selection(arguments, "signal_offset", "signal_limit", compact)?;
    let mut metadata = serde_json::Map::new();
    paginate_findings(&mut value, findings, &mut metadata)?;
    paginate_signals(&mut value, signals, &mut metadata)?;
    let text = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("paginate failed: {error}"))?;
    Ok(super::analysis_store::PaginatedReport {
        text,
        metadata: (!metadata.is_empty()).then_some(Value::Object(metadata)),
    })
}

#[derive(Clone, Copy)]
struct PageSelection {
    offset: usize,
    limit: Option<usize>,
}

fn page_selection(
    arguments: &Value,
    offset_key: &str,
    limit_key: &str,
    compact: bool,
) -> Result<PageSelection, String> {
    let offset = parse_usize(arguments, offset_key)?.unwrap_or(0);
    let default_limit = (compact || arguments.get(offset_key).is_some()).then_some(if compact {
        COMPACT_LIMIT
    } else {
        DEFAULT_PAGE_LIMIT
    });
    Ok(PageSelection {
        offset,
        limit: parse_usize(arguments, limit_key)?.or(default_limit),
    })
}

fn paginate_findings(
    value: &mut Value,
    page: PageSelection,
    metadata: &mut serde_json::Map<String, Value>,
) -> Result<(), String> {
    let Some(limit) = page.limit else {
        return Ok(());
    };
    validate_limit("limit", limit)?;
    let findings = value
        .get_mut("findings")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "review report has no findings array".to_string())?;
    let total = findings.len();
    let (start, end) = page_bounds(page.offset, limit, total);
    *findings = findings[start..end].to_vec();
    metadata.extend([
        ("offset".to_string(), json!(page.offset)),
        ("limit".to_string(), json!(limit)),
        ("total".to_string(), json!(total)),
        ("returned".to_string(), json!(end - start)),
        (
            "next_offset".to_string(),
            json!((end < total).then_some(end)),
        ),
    ]);
    Ok(())
}

fn paginate_signals(
    value: &mut Value,
    page: PageSelection,
    metadata: &mut serde_json::Map<String, Value>,
) -> Result<(), String> {
    let Some(limit) = page.limit else {
        return Ok(());
    };
    validate_limit("signal_limit", limit)?;
    let tiers = value
        .get_mut("tiered_signals")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "review report has no tiered_signals object".to_string())?;
    let tier_names = ["definitely", "maybe", "noise"];
    let flattened = flatten_signals(tiers, &tier_names);
    let total = flattened.len();
    let (start, end) = page_bounds(page.offset, limit, total);
    let mut paged_tiers = vec![Vec::new(); tier_names.len()];
    for (tier_index, signal) in flattened.into_iter().skip(start).take(end - start) {
        paged_tiers[tier_index].push(signal);
    }
    for (tier, signals) in tier_names.into_iter().zip(paged_tiers) {
        tiers.insert(tier.to_string(), json!(signals));
    }
    metadata.insert(
        "signals".to_string(),
        page_metadata(page.offset, limit, total, end),
    );
    Ok(())
}

fn flatten_signals(tiers: &serde_json::Map<String, Value>, names: &[&str]) -> Vec<(usize, Value)> {
    names
        .iter()
        .enumerate()
        .flat_map(|(index, tier)| {
            tiers
                .get(*tier)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(move |signal| (index, signal.clone()))
        })
        .collect()
}

fn page_bounds(offset: usize, limit: usize, total: usize) -> (usize, usize) {
    let start = offset.min(total);
    (start, start.saturating_add(limit).min(total))
}

fn page_metadata(offset: usize, limit: usize, total: usize, end: usize) -> Value {
    json!({
        "offset": offset,
        "limit": limit,
        "total": total,
        "returned": end.saturating_sub(offset.min(total)),
        "next_offset": (end < total).then_some(end)
    })
}

fn parse_usize(arguments: &Value, key: &str) -> Result<Option<usize>, String> {
    let Some(value) = arguments.get(key) else {
        return Ok(None);
    };
    let raw = value
        .as_u64()
        .ok_or_else(|| format!("`{key}` must be a non-negative integer"))?;
    usize::try_from(raw)
        .map(Some)
        .map_err(|_| format!("`{key}` is too large"))
}

fn validate_limit(key: &str, limit: usize) -> Result<(), String> {
    if limit == 0 || limit > MAX_PAGE_LIMIT {
        Err(format!("`{key}` must be between 1 and {MAX_PAGE_LIMIT}"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::paginate_review_json;
    use serde_json::{Value, json};

    #[test]
    fn compact_review_paginates_findings_and_signals_with_totals() {
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
            "findings": [1, 2],
            "tiered_signals": {"definitely": ["a", "b"], "maybe": ["c"], "noise": ["d"]}
        }"#;

        let page =
            paginate_review_json(rendered, &json!({ "signal_offset": 1, "signal_limit": 2 }))
                .expect("paginate review");
        let report: Value = serde_json::from_str(&page.text).expect("report JSON");
        assert!(page.text.contains("pytest-node-v1"));
        assert!(
            page.text
                .contains("python.tests:tests/test_api.py::test_create:failed")
        );
        assert_eq!(report["findings"], json!([1, 2]));
        assert_eq!(report["tiered_signals"]["definitely"], json!(["b"]));
        assert_eq!(report["tiered_signals"]["maybe"], json!(["c"]));
        assert_eq!(
            page.metadata.as_ref().expect("pagination")["signals"]["total"],
            4
        );
        assert_eq!(
            page.metadata.as_ref().expect("pagination")["signals"]["next_offset"],
            3
        );
    }
}
