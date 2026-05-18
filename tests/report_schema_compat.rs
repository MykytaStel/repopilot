use serde_json::Value;

#[test]
fn schema_fixtures_document_legacy_and_current_metric_names() {
    let legacy: Value = serde_json::from_str(include_str!("fixtures/reports/scan-v010.json"))
        .expect("legacy report fixture should be valid JSON");
    let current: Value = serde_json::from_str(include_str!("fixtures/reports/scan-v013.json"))
        .expect("current report fixture should be valid JSON");

    assert_eq!(legacy["schema_version"], "0.10");
    assert_eq!(legacy["files_count"], 2);
    assert_eq!(legacy["lines_of_code"], 12);

    assert_eq!(current["schema_version"], "0.13");
    assert_eq!(current["files_analyzed"], 2);
    assert_eq!(current["non_empty_lines"], 12);
    assert_eq!(current["large_files_skipped"], 0);
}

#[test]
fn legacy_metric_fallbacks_are_unambiguous_for_consumers() {
    let legacy: Value = serde_json::from_str(include_str!("fixtures/reports/scan-v010.json"))
        .expect("legacy report fixture should be valid JSON");

    let files_analyzed = legacy
        .get("files_analyzed")
        .or_else(|| legacy.get("files_count"))
        .and_then(Value::as_u64)
        .expect("files analyzed fallback");

    let non_empty_lines = legacy
        .get("non_empty_lines")
        .or_else(|| legacy.get("lines_of_code"))
        .and_then(Value::as_u64)
        .expect("non-empty lines fallback");

    assert_eq!(files_analyzed, 2);
    assert_eq!(non_empty_lines, 12);
}
