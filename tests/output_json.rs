use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::scan::types::{LanguageSummary, ScanSummary};
use std::path::PathBuf;

#[test]
fn renders_valid_json_scan_summary() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        files_count: 1,
        directories_count: 0,
        lines_of_code: 3,
        languages: vec![LanguageSummary {
            name: "Rust".to_string(),
            files_count: 1,
        }],
        markers: vec![],
    };

    let output =
        render_scan_summary(&summary, OutputFormat::Json).expect("failed to render json summary");

    let parsed: serde_json::Value =
        serde_json::from_str(&output).expect("output should be valid json");

    assert_eq!(parsed["root_path"], "demo");
    assert_eq!(parsed["files_count"], 1);
    assert_eq!(parsed["directories_count"], 0);
    assert_eq!(parsed["lines_of_code"], 3);
    assert_eq!(parsed["languages"][0]["name"], "Rust");
}
