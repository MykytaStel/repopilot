use repopilot::report::schema::{
    SCAN_REPORT_SCHEMA_VERSION, parse_scan_summary_json, parse_scan_summary_value,
};
use serde_json::Value;

#[test]
fn current_schema_fixture_documents_scan_report_contract() {
    let current_text = include_str!("fixtures/reports/scan-v022.json");
    let current: Value =
        serde_json::from_str(current_text).expect("current report fixture should be valid JSON");

    assert_eq!(SCAN_REPORT_SCHEMA_VERSION, "0.22");
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
    assert_eq!(current["raw_findings_count"], 0);
    assert_eq!(current["raw_signal_quality"]["findings_total"], 0);
    assert_eq!(current["visible_signal_quality"]["findings_total"], 0);
    assert_eq!(current["context_graph_summary"]["files"], 2);
    assert_eq!(current["context_graph_cache"]["status"], "write");
}

#[test]
fn strict_reader_accepts_current_scan_report_shape() {
    let current_text = include_str!("fixtures/reports/scan-v022.json");
    let current = parse_scan_summary_json(current_text)
        .expect("current report should parse into ScanSummary");

    assert_eq!(current.metrics.files_discovered, 2);
    assert_eq!(current.metrics.files_analyzed, 2);
    assert_eq!(current.metrics.non_empty_lines, 12);
    assert_eq!(current.artifacts.diagnostics.len(), 1);
    assert_eq!(
        current.artifacts.diagnostics[0].code,
        "workspace.package-scan-failed"
    );
    assert_eq!(
        current
            .artifacts
            .context_graph_summary
            .as_ref()
            .map(|graph| graph.files),
        Some(2)
    );
    assert_eq!(
        current
            .artifacts
            .context_graph_cache
            .as_ref()
            .map(|cache| cache.status.as_str()),
        Some("write")
    );
}

#[test]
fn strict_reader_accepts_previous_scan_report_shapes() {
    let previous_v021 = parse_scan_summary_json(include_str!("fixtures/reports/scan-v021.json"))
        .expect("0.21 report should parse during 0.22 transition");
    let previous_v020 = parse_scan_summary_json(include_str!("fixtures/reports/scan-v020.json"))
        .expect("0.20 report should parse during 0.22 transition");
    let previous_v019 = parse_scan_summary_json(include_str!("fixtures/reports/scan-v019.json"))
        .expect("0.19 report should parse during 0.22 transition");
    let previous_v018 = parse_scan_summary_json(include_str!("fixtures/reports/scan-v018.json"))
        .expect("0.18 report should parse during 0.22 transition");
    let previous_v017 = parse_scan_summary_json(include_str!("fixtures/reports/scan-v017.json"))
        .expect("0.17 report should parse during 0.22 transition");
    let previous_v016 = parse_scan_summary_json(include_str!("fixtures/reports/scan-v016.json"))
        .expect("0.16 report should parse during 0.22 transition");

    assert_eq!(previous_v021.metrics.files_discovered, 2);
    assert_eq!(previous_v020.metrics.files_discovered, 2);
    assert_eq!(previous_v019.metrics.files_discovered, 2);
    assert_eq!(previous_v018.metrics.files_discovered, 2);
    assert_eq!(previous_v017.metrics.files_discovered, 2);
    assert_eq!(previous_v016.metrics.files_discovered, 2);
    assert_eq!(
        previous_v016
            .artifacts
            .context_graph_summary
            .as_ref()
            .map(|graph| graph.files),
        Some(2)
    );
}

#[test]
fn strict_reader_ignores_occurrence_key_and_decision_fields_on_findings() {
    // A finding carrying the v0.21 `occurrence_key`/`decision` fields must
    // still deserialize into `Finding` — proving the addition is genuinely
    // additive for readers that don't know about it yet.
    let mut report: Value =
        serde_json::from_str(include_str!("fixtures/reports/scan-v022.json")).expect("valid JSON");
    report["findings"] = serde_json::json!([{
        "id": "rule.example:src/lib.rs:deadbeef",
        "rule_id": "rule.example",
        "title": "Example",
        "description": "desc",
        "recommendation": "fix it",
        "category": "SECURITY",
        "severity": "HIGH",
        "confidence": "HIGH",
        "evidence": [],
        "provenance": {
            "detector": "rule.example",
            "signal_source": "ast",
            "rule_lifecycle": "stable",
            "analysis_scope": "file"
        },
        "risk": { "score": 0, "priority": "P3", "signals": [], "formula_version": "v1" },
        "occurrence_key": "abcdef0123456789",
        "decision": {
            "severity": "HIGH",
            "confidence": "HIGH",
            "evidence": [],
            "recommendation": "fix it"
        }
    }]);

    let parsed = parse_scan_summary_value(report).expect("new-shape finding should still parse");
    assert_eq!(parsed.artifacts.findings.len(), 1);
    assert_eq!(
        parsed.artifacts.findings[0].id,
        "rule.example:src/lib.rs:deadbeef"
    );
}

#[test]
fn strict_reader_rejects_legacy_report_shapes() {
    let legacy = parse_scan_summary_json(include_str!("fixtures/reports/scan-v010.json"));
    let previous_envelope =
        parse_scan_summary_json(include_str!("fixtures/reports/scan-v014.json"));
    let previous_v015 = parse_scan_summary_json(include_str!("fixtures/reports/scan-v015.json"));

    assert!(legacy.is_err());
    assert!(previous_envelope.is_err());
    assert!(previous_v015.is_err());
}
