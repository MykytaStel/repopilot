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
    let finding_offset = parse_usize(arguments, "offset")?.unwrap_or(0);
    let finding_limit = parse_usize(arguments, "limit")?.or_else(|| {
        (compact || arguments.get("offset").is_some()).then_some(if compact {
            COMPACT_LIMIT
        } else {
            DEFAULT_PAGE_LIMIT
        })
    });
    let signal_offset = parse_usize(arguments, "signal_offset")?.unwrap_or(0);
    let signal_limit = parse_usize(arguments, "signal_limit")?.or_else(|| {
        (compact || arguments.get("signal_offset").is_some()).then_some(if compact {
            COMPACT_LIMIT
        } else {
            DEFAULT_PAGE_LIMIT
        })
    });
    let mut metadata = serde_json::Map::new();

    if let Some(limit) = finding_limit {
        validate_limit("limit", limit)?;
        let findings = value
            .get_mut("findings")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| "review report has no findings array".to_string())?;
        let total = findings.len();
        let start = finding_offset.min(total);
        let end = start.saturating_add(limit).min(total);
        let page = findings[start..end].to_vec();
        *findings = page;
        metadata.extend([
            ("offset".to_string(), json!(finding_offset)),
            ("limit".to_string(), json!(limit)),
            ("total".to_string(), json!(total)),
            ("returned".to_string(), json!(end - start)),
            (
                "next_offset".to_string(),
                json!((end < total).then_some(end)),
            ),
        ]);
    }

    if let Some(limit) = signal_limit {
        validate_limit("signal_limit", limit)?;
        let tiers = value
            .get_mut("tiered_signals")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "review report has no tiered_signals object".to_string())?;
        let tier_names = ["definitely", "maybe", "noise"];
        let total = tier_names
            .iter()
            .filter_map(|tier| tiers.get(*tier).and_then(Value::as_array))
            .map(Vec::len)
            .sum::<usize>();
        let start = signal_offset.min(total);
        let end = start.saturating_add(limit).min(total);
        let flattened = tier_names
            .iter()
            .enumerate()
            .flat_map(|(tier_index, tier)| {
                tiers
                    .get(*tier)
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(move |signal| (tier_index, signal.clone()))
            })
            .collect::<Vec<_>>();
        let mut page = vec![Vec::new(); tier_names.len()];
        for (tier_index, signal) in flattened.into_iter().skip(start).take(end - start) {
            page[tier_index].push(signal);
        }
        for (tier, signals) in tier_names.into_iter().zip(page) {
            tiers.insert(tier.to_string(), json!(signals));
        }
        metadata.insert(
            "signals".to_string(),
            json!({
                "offset": signal_offset,
                "limit": limit,
                "total": total,
                "returned": end - start,
                "next_offset": (end < total).then_some(end)
            }),
        );
    }

    let text = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("paginate failed: {error}"))?;
    Ok(super::analysis_store::PaginatedReport {
        text,
        metadata: (!metadata.is_empty()).then_some(Value::Object(metadata)),
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
