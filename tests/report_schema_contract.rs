use repopilot::report::schema::{SCAN_REPORT_SCHEMA_VERSION, parse_scan_summary_json};
use serde_json::Value;

#[test]
fn current_schema_fixture_documents_scan_report_contract() {
    let current: Value = serde_json::from_str(include_str!("fixtures/reports/scan-v016.json"))
        .expect("current report fixture should be valid JSON");

    assert_eq!(SCAN_REPORT_SCHEMA_VERSION, "0.16");
    assert_eq!(current["schema_version"], SCAN_REPORT_SCHEMA_VERSION);
    assert_eq!(current["report"]["kind"], "scan");
    assert_eq!(
        current["report"]["schema_version"],
        SCAN_REPORT_SCHEMA_VERSION
    );
    assert_eq!(current["files_discovered"], 2);
    assert_eq!(current["files_analyzed"], 2);
    assert_eq!(current["non_empty_lines"], 12);
    assert_eq!(current["large_files_skipped"], 0);
    assert_eq!(
        current["diagnostics"][0]["code"],
        "workspace.package-scan-failed"
    );
    assert_eq!(current["signal_quality"]["findings_total"], 0);
    assert_eq!(current["signal_quality"]["evidence_coverage_percent"], 100);
    assert_eq!(current["context_graph_summary"]["files"], 2);
    assert_eq!(current["context_graph_cache"]["status"], "write");
}

#[test]
fn strict_reader_accepts_current_scan_report_shape() {
    let current = parse_scan_summary_json(include_str!("fixtures/reports/scan-v016.json"))
        .expect("current report should parse into ScanSummary");

    assert_eq!(current.files_discovered, 2);
    assert_eq!(current.files_analyzed, 2);
    assert_eq!(current.non_empty_lines, 12);
    assert_eq!(current.diagnostics.len(), 1);
    assert_eq!(current.diagnostics[0].code, "workspace.package-scan-failed");
    assert_eq!(
        current
            .context_graph_summary
            .as_ref()
            .map(|graph| graph.files),
        Some(2)
    );
    assert_eq!(
        current
            .context_graph_cache
            .as_ref()
            .map(|cache| cache.status.as_str()),
        Some("write")
    );
}

#[test]
fn strict_reader_accepts_previous_scan_report_shape() {
    let previous = parse_scan_summary_json(include_str!("fixtures/reports/scan-v015.json"))
        .expect("0.15 report should parse during 0.16 transition");

    assert_eq!(previous.files_discovered, 2);
    assert!(previous.context_graph_summary.is_none());
}

#[test]
fn strict_reader_rejects_legacy_report_shapes() {
    let legacy = parse_scan_summary_json(include_str!("fixtures/reports/scan-v010.json"));
    let previous_envelope =
        parse_scan_summary_json(include_str!("fixtures/reports/scan-v014.json"));

    assert!(legacy.is_err());
    assert!(previous_envelope.is_err());
}
