use repopilot::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use repopilot::frameworks::ReactNativeArchitectureProfile;
use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::risk::{RiskInputs, RiskSummary, assess_finding};
use repopilot::scan::types::{LanguageSummary, ScanSummary};
use std::path::PathBuf;

#[test]
fn renders_valid_json_scan_summary() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        files_discovered: 0,
        files_count: 1,
        directories_count: 0,
        lines_of_code: 3,
        skipped_files_count: 0,
        files_skipped_low_signal: 0,
        binary_files_skipped: 0,
        skipped_bytes: 0,
        languages: vec![LanguageSummary {
            name: "Rust".to_string(),
            files_count: 1,
        }],
        findings: vec![],
        detected_frameworks: vec![],
        framework_projects: vec![],
        react_native: None,
        coupling_graph: None,
        scan_duration_us: 0,
        health_score: 0,
        visible_findings_count: 0,
        hidden_suggestions_count: 0,
        visibility_profile: None,
        files_skipped_by_limit: 0,
        files_skipped_repopilotignore: 0,
        repopilotignore_path: None,
        scan_timings: None,
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
    assert_eq!(parsed["risk_summary"]["total"], 0);
    assert_eq!(parsed["risk_summary"]["average_score"], 0);
    // react_native must be absent when None
    assert!(parsed.get("react_native").is_none());
}

#[test]
fn json_findings_include_confidence() {
    let mut finding = Finding {
        id: "code-quality.long-function:src/lib.rs:1".to_string(),
        rule_id: "code-quality.long-function".to_string(),
        recommendation: Finding::recommendation_for_rule_id("code-quality.long-function"),
        title: "Large Rust production function".to_string(),
        description: "Function spans more lines than the configured threshold.".to_string(),
        category: FindingCategory::CodeQuality,
        severity: Severity::Medium,
        confidence: Confidence::High,
        evidence: vec![Evidence {
            path: PathBuf::from("src/lib.rs"),
            line_start: 1,
            line_end: Some(80),
            snippet: "function spans lines 1-80".to_string(),
        }],
        workspace_package: None,
        docs_url: None,
        risk: Default::default(),
    };
    finding.risk = assess_finding(&finding, None, RiskInputs::default());

    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        findings: vec![finding],
        ..ScanSummary::default()
    };
    let risk_summary = RiskSummary::from_findings(&summary.findings);
    assert_eq!(risk_summary.total, 1);
    assert_eq!(risk_summary.counts.p2, 1);

    let output =
        render_scan_summary(&summary, OutputFormat::Json).expect("failed to render json summary");
    let parsed: serde_json::Value =
        serde_json::from_str(&output).expect("output should be valid json");

    assert_eq!(parsed["findings"][0]["confidence"], "HIGH");
    assert_eq!(parsed["risk_summary"]["total"], 1);
    assert_eq!(parsed["risk_summary"]["counts"]["p2"], 1);
    assert_eq!(parsed["risk_summary"]["highest_priority"], "P2");
    assert_eq!(parsed["risk_summary"]["average_score"], 50);
    assert_eq!(parsed["findings"][0]["risk"]["priority"], "P2");
    assert_eq!(parsed["findings"][0]["risk"]["score"], 50);
    assert_eq!(
        parsed["findings"][0]["risk"]["signals"][0]["id"],
        "severity.medium"
    );
    assert_eq!(
        parsed["findings"][0]["recommendation"],
        Finding::recommendation_for_rule_id("code-quality.long-function")
    );
}

#[test]
fn react_native_profile_appears_in_json_when_present() {
    let summary = ScanSummary {
        root_path: PathBuf::from("rn-app"),
        react_native: Some(ReactNativeArchitectureProfile {
            detected: true,
            react_native_version: Some("0.73.0".to_string()),
            has_ios: true,
            has_android: true,
            has_metro_config: true,
            has_react_native_config: false,
            has_expo_config: true,
            has_codegen_config: true,
            expo_new_arch_enabled: Some(true),
            android_new_arch_enabled: Some(true),
            ios_new_arch_enabled: None,
            android_hermes_enabled: Some(true),
            hermes_enabled: Some(true),
            package_manager: Some("npm".to_string()),
            android_gradle_properties_found: true,
            ios_podfile_found: false,
            ..ReactNativeArchitectureProfile::default()
        }),
        ..ScanSummary::default()
    };

    let output =
        render_scan_summary(&summary, OutputFormat::Json).expect("failed to render json summary");

    let parsed: serde_json::Value =
        serde_json::from_str(&output).expect("output should be valid json");

    let rn = &parsed["react_native"];
    assert_eq!(rn["detected"], true);
    assert_eq!(rn["react_native_version"], "0.73.0");
    assert_eq!(rn["has_ios"], true);
    assert_eq!(rn["has_android"], true);
    assert_eq!(rn["android_new_arch_enabled"], true);
    assert_eq!(rn["expo_new_arch_enabled"], true);
    assert!(rn["ios_new_arch_enabled"].is_null());
    assert_eq!(rn["hermes_enabled"], true);
    assert_eq!(rn["package_manager"], "npm");
    assert_eq!(rn["has_codegen_config"], true);
}
