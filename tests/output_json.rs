use repopilot::frameworks::ReactNativeArchitectureProfile;
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
        skipped_files_count: 0,
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
    // react_native must be absent when None
    assert!(parsed.get("react_native").is_none());
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
