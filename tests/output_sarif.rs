use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::scan::types::ScanSummary;
use std::path::PathBuf;

#[test]
fn renders_valid_sarif_scan_summary() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        ..ScanSummary::default()
    };

    let output =
        render_scan_summary(&summary, OutputFormat::Sarif).expect("failed to render SARIF summary");

    let parsed: serde_json::Value =
        serde_json::from_str(&output).expect("SARIF output should be valid JSON");

    assert_eq!(parsed["version"], "2.1.0");
    assert!(parsed["runs"].is_array());
}
