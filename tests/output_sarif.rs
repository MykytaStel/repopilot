use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::output::sarif::findings_to_sarif;
use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::scan::types::ScanSummary;
use std::path::PathBuf;

fn make_finding_with_package(rule_id: &str, pkg: Option<&str>) -> Finding {
    Finding {
        id: String::new(),
        rule_id: rule_id.to_owned(),
        recommendation: Finding::recommendation_for_rule_id(rule_id),
        title: "Test".to_owned(),
        description: "desc".to_owned(),
        category: FindingCategory::Security,
        severity: Severity::High,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from("src/main.rs"),
            line_start: 1,
            line_end: None,
            snippet: String::new(),
        }],
        workspace_package: pkg.map(str::to_owned),
        docs_url: None,
        risk: Default::default(),
    }
}

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
fn sarif_rule_uses_title_and_full_description() {
    let finding = Finding {
        id: String::new(),
        rule_id: "test.rule".to_string(),
        recommendation: Finding::recommendation_for_rule_id("test.rule"),
        title: "Test finding".to_string(),
        description: "A real description for this rule.".to_string(),
        category: FindingCategory::CodeQuality,
        severity: Severity::Medium,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from("src/main.rs"),
            line_start: 10,
            line_end: None,
            snippet: "let x = 1;".to_string(),
        }],
        workspace_package: None,
        docs_url: None,
        risk: Default::default(),
    };

    let root = PathBuf::from(".");
    let sarif = findings_to_sarif(&[finding], &root);
    let serialized = serde_json::to_value(&sarif).unwrap();

    let short_desc =
        &serialized["runs"][0]["tool"]["driver"]["rules"][0]["shortDescription"]["text"];
    let full_desc = &serialized["runs"][0]["tool"]["driver"]["rules"][0]["fullDescription"]["text"];
    assert_eq!(
        short_desc.as_str().unwrap(),
        "Test finding",
        "rule shortDescription should use the finding title"
    );
    assert_eq!(
        full_desc.as_str().unwrap(),
        "A real description for this rule.",
        "rule fullDescription should use the actual finding description"
    );
    assert!(
        !full_desc.as_str().unwrap().starts_with("RepoPilot rule"),
        "should not use the generic fallback description"
    );
}

#[test]
fn sarif_result_includes_partial_fingerprints() {
    let finding = Finding {
        id: String::new(),
        rule_id: "test.rule".to_string(),
        recommendation: Finding::recommendation_for_rule_id("test.rule"),
        title: "Test".to_string(),
        description: "desc".to_string(),
        category: FindingCategory::CodeQuality,
        severity: Severity::Low,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from("src/lib.rs"),
            line_start: 5,
            line_end: None,
            snippet: String::new(),
        }],
        workspace_package: None,
        docs_url: None,
        risk: Default::default(),
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

#[test]
fn sarif_result_properties_include_category() {
    let finding = make_finding_with_package("security.secret-candidate", None);
    let sarif = findings_to_sarif(&[finding], &PathBuf::from("."));
    let value = serde_json::to_value(&sarif).unwrap();

    let category = &value["runs"][0]["results"][0]["properties"]["category"];
    assert_eq!(
        category.as_str(),
        Some("security"),
        "result properties must include the finding category label"
    );
}

#[test]
fn sarif_result_properties_include_confidence() {
    let finding = make_finding_with_package("security.secret-candidate", None);
    let sarif = findings_to_sarif(&[finding], &PathBuf::from("."));
    let value = serde_json::to_value(&sarif).unwrap();

    let confidence = &value["runs"][0]["results"][0]["properties"]["confidence"];
    assert_eq!(
        confidence.as_str(),
        Some("MEDIUM"),
        "result properties must include the finding confidence label"
    );
}

#[test]
fn sarif_result_properties_include_recommendation() {
    let finding = make_finding_with_package("security.secret-candidate", None);
    let expected = finding.recommendation.clone();
    let sarif = findings_to_sarif(&[finding], &PathBuf::from("."));
    let value = serde_json::to_value(&sarif).unwrap();

    let recommendation = &value["runs"][0]["results"][0]["properties"]["recommendation"];
    assert_eq!(
        recommendation.as_str(),
        Some(expected.as_str()),
        "result properties must include the finding recommendation"
    );
}

#[test]
fn sarif_result_properties_include_workspace_package() {
    let finding = make_finding_with_package("security.env-file-committed", Some("web"));
    let sarif = findings_to_sarif(&[finding], &PathBuf::from("."));
    let value = serde_json::to_value(&sarif).unwrap();

    let pkg = &value["runs"][0]["results"][0]["properties"]["workspacePackage"];
    assert_eq!(
        pkg.as_str(),
        Some("web"),
        "workspacePackage must be serialized in result properties"
    );
}

#[test]
fn sarif_rule_help_text_from_registry() {
    let finding = make_finding_with_package("security.secret-candidate", None);
    let sarif = findings_to_sarif(&[finding], &PathBuf::from("."));
    let value = serde_json::to_value(&sarif).unwrap();

    let help_text = &value["runs"][0]["tool"]["driver"]["rules"][0]["help"]["text"];
    assert!(
        help_text.is_string(),
        "rule help.text must be present for a rule with a recommendation"
    );
    assert!(
        !help_text.as_str().unwrap().is_empty(),
        "rule help.text must not be empty"
    );
}
