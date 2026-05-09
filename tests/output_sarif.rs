use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::output::sarif::findings_to_sarif;
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

#[test]
fn sarif_driver_includes_version() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        ..ScanSummary::default()
    };

    let output =
        render_scan_summary(&summary, OutputFormat::Sarif).expect("failed to render SARIF");
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let version = &parsed["runs"][0]["tool"]["driver"]["version"];
    assert!(version.is_string(), "tool.driver.version should be present");
    assert!(!version.as_str().unwrap().is_empty());
}

#[test]
fn sarif_rule_uses_real_description_not_generic() {
    let finding = Finding {
        id: String::new(),
        rule_id: "test.rule".to_string(),
        title: "Test finding".to_string(),
        description: "A real description for this rule.".to_string(),
        category: FindingCategory::CodeQuality,
        severity: Severity::Medium,
        evidence: vec![Evidence {
            path: PathBuf::from("src/main.rs"),
            line_start: 10,
            line_end: None,
            snippet: "let x = 1;".to_string(),
        }],
        workspace_package: None,
        docs_url: None,
    };

    let root = PathBuf::from(".");
    let sarif = findings_to_sarif(&[finding], &root);
    let serialized = serde_json::to_value(&sarif).unwrap();

    let rule_desc =
        &serialized["runs"][0]["tool"]["driver"]["rules"][0]["shortDescription"]["text"];
    assert_eq!(
        rule_desc.as_str().unwrap(),
        "A real description for this rule.",
        "rule shortDescription should use the actual finding description"
    );
    assert!(
        !rule_desc.as_str().unwrap().starts_with("RepoPilot rule"),
        "should not use the generic fallback description"
    );
}

#[test]
fn sarif_result_includes_partial_fingerprints() {
    let finding = Finding {
        id: String::new(),
        rule_id: "test.rule".to_string(),
        title: "Test".to_string(),
        description: "desc".to_string(),
        category: FindingCategory::CodeQuality,
        severity: Severity::Low,
        evidence: vec![Evidence {
            path: PathBuf::from("src/lib.rs"),
            line_start: 5,
            line_end: None,
            snippet: String::new(),
        }],
        workspace_package: None,
        docs_url: None,
    };

    let root = PathBuf::from(".");
    let sarif = findings_to_sarif(&[finding], &root);
    let serialized = serde_json::to_value(&sarif).unwrap();

    let fingerprints = &serialized["runs"][0]["results"][0]["partialFingerprints"];
    assert!(
        fingerprints.is_object(),
        "partialFingerprints should be present on results with evidence"
    );
    assert!(
        fingerprints["primaryLocationLineHash/v1"].is_string(),
        "primaryLocationLineHash/v1 key should be present"
    );
}
