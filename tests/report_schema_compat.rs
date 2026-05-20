use repopilot::report::schema::{SCAN_REPORT_SCHEMA_VERSION, parse_scan_summary_json};
use serde_json::Value;

#[test]
fn schema_fixtures_document_legacy_and_013_metric_names() {
    let legacy: Value = serde_json::from_str(include_str!("fixtures/reports/scan-v010.json"))
        .expect("legacy report fixture should be valid JSON");
    let v013: Value = serde_json::from_str(include_str!("fixtures/reports/scan-v013.json"))
        .expect("0.13 report fixture should be valid JSON");

    assert_eq!(legacy["schema_version"], "0.10");
    assert_eq!(legacy["files_count"], 2);
    assert_eq!(legacy["lines_of_code"], 12);

    assert_eq!(SCAN_REPORT_SCHEMA_VERSION, "0.14");
    assert_eq!(v013["schema_version"], "0.13");
    assert_eq!(v013["report"]["kind"], "scan");
    assert_eq!(v013["report"]["schema_version"], "0.13");
    assert_eq!(v013["files_analyzed"], 2);
    assert_eq!(v013["non_empty_lines"], 12);
    assert_eq!(v013["large_files_skipped"], 0);
    assert_eq!(
        v013["diagnostics"][0]["code"],
        "workspace.package-scan-failed"
    );
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

#[test]
fn migration_reader_accepts_legacy_and_current_report_shapes() {
    let legacy = parse_scan_summary_json(include_str!("fixtures/reports/scan-v010.json"))
        .expect("legacy report should migrate into ScanSummary");
    let current = parse_scan_summary_json(include_str!("fixtures/reports/scan-v013.json"))
        .expect("current report should parse into ScanSummary");

    assert_eq!(legacy.files_analyzed, 2);
    assert_eq!(legacy.non_empty_lines, 12);
    assert_eq!(current.files_analyzed, 2);
    assert_eq!(current.non_empty_lines, 12);
    assert_eq!(current.diagnostics.len(), 1);
    assert_eq!(current.diagnostics[0].code, "workspace.package-scan-failed");
}
