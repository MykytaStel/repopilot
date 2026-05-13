use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::frameworks::ReactNativeArchitectureProfile;
use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::scan::types::{LanguageSummary, ScanSummary};
use std::path::PathBuf;

#[test]
fn renders_markdown_scan_summary() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo-project"),
        files_discovered: 0,
        files_count: 2,
        directories_count: 1,
        lines_of_code: 10,
        skipped_files_count: 0,
        files_skipped_low_signal: 0,
        binary_files_skipped: 0,
        skipped_bytes: 0,
        languages: vec![
            LanguageSummary {
                name: "Rust".to_string(),
                files_count: 1,
            },
            LanguageSummary {
                name: "TypeScript".to_string(),
                files_count: 1,
            },
        ],
        findings: vec![Finding {
            id: "code-marker.todo.src/main.rs:7".to_string(),
            rule_id: "code-marker.todo".to_string(),
            recommendation: Finding::recommendation_for_rule_id("code-marker.todo"),
            title: "TODO marker found".to_string(),
            description: "A TODO marker was found in the codebase and should be reviewed."
                .to_string(),
            category: FindingCategory::CodeQuality,
            severity: Severity::Low,
            confidence: Default::default(),
            evidence: vec![Evidence {
                path: PathBuf::from("src/main.rs"),
                line_start: 7,
                line_end: None,
                snippet: "// TODO: improve architecture".to_string(),
            }],
            workspace_package: None,
            docs_url: None,
        }],
        detected_frameworks: vec![],
        framework_projects: vec![],
        react_native: None,
        coupling_graph: None,
        scan_duration_us: 0,
        health_score: 0,
        files_skipped_by_limit: 0,
        files_skipped_repopilotignore: 0,
        repopilotignore_path: None,
    };

    let output = render_scan_summary(&summary, OutputFormat::Markdown)
        .expect("failed to render markdown summary");

    assert!(output.contains("# RepoPilot Scan Report"));
    assert!(output.contains(&format!(
        "- **RepoPilot version:** {}",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(output.contains("## Overview"));
    assert!(output.contains("## Risk Summary"));
    assert!(output.contains("## Top Rules"));
    assert!(output.contains("## Languages"));
    assert!(output.contains("## Findings Index"));
    assert!(output.contains("## Findings"));
    assert!(output.contains("### Code Quality"));
    assert!(output.contains("#### `code-marker.todo` (1)"));
    assert!(output.contains("| `code-marker.todo` | 1 | LOW |"));
    assert!(output.contains("`src/main.rs:7`"));
    assert!(output.contains("- **Path:** `demo-project`"));
    assert!(output.contains("- **Files analyzed:** 2"));
    assert!(output.contains("| Rust | 1 |"));
    assert!(output.contains("| TypeScript | 1 |"));
    assert!(output.contains("Evidence: `src/main.rs:7` - // TODO: improve architecture"));
    assert!(output.contains("Recommendation:"));
    assert!(output.contains("Convert the TODO into a tracked issue"));
}

#[test]
fn renders_empty_markdown_sections() {
    let summary = ScanSummary {
        root_path: PathBuf::from("empty-project"),
        files_discovered: 0,
        files_count: 0,
        directories_count: 0,
        lines_of_code: 0,
        skipped_files_count: 0,
        files_skipped_low_signal: 0,
        binary_files_skipped: 0,
        skipped_bytes: 0,
        languages: vec![],
        findings: vec![],
        detected_frameworks: vec![],
        framework_projects: vec![],
        react_native: None,
        coupling_graph: None,
        scan_duration_us: 0,
        health_score: 0,
        files_skipped_by_limit: 0,
        files_skipped_repopilotignore: 0,
        repopilotignore_path: None,
    };

    let output = render_scan_summary(&summary, OutputFormat::Markdown)
        .expect("failed to render markdown summary");

    assert!(output.contains("No languages detected."));
    assert!(output.contains("No findings found."));
    assert!(output.contains("No rules triggered."));
    // Non-RN project must not render the React Native architecture section
    assert!(!output.contains("### React Native"));
}

#[test]
fn renders_react_native_architecture_section_when_profile_present() {
    let summary = ScanSummary {
        root_path: PathBuf::from("rn-project"),
        files_discovered: 0,
        files_count: 5,
        directories_count: 2,
        lines_of_code: 100,
        skipped_files_count: 0,
        files_skipped_low_signal: 0,
        binary_files_skipped: 0,
        skipped_bytes: 0,
        languages: vec![],
        findings: vec![],
        detected_frameworks: vec![],
        framework_projects: vec![],
        react_native: Some(ReactNativeArchitectureProfile {
            detected: true,
            react_native_version: Some("0.73.0".to_string()),
            has_ios: true,
            has_android: true,
            has_metro_config: false,
            has_react_native_config: false,
            has_codegen_config: false,
            android_new_arch_enabled: Some(false),
            ios_new_arch_enabled: None,
            hermes_enabled: Some(true),
            android_gradle_properties_found: true,
            ios_podfile_found: false,
            ..ReactNativeArchitectureProfile::default()
        }),
        coupling_graph: None,
        scan_duration_us: 0,
        health_score: 0,
        files_skipped_by_limit: 0,
        files_skipped_repopilotignore: 0,
        repopilotignore_path: None,
    };

    let output = render_scan_summary(&summary, OutputFormat::Markdown)
        .expect("failed to render markdown summary");

    assert!(output.contains("### React Native"));
    assert!(output.contains("0.73.0"));
    assert!(output.contains("**iOS:** detected"));
    assert!(output.contains("**Android:** detected"));
    assert!(output.contains("**Android New Architecture:** disabled"));
    assert!(output.contains("**iOS New Architecture:** unknown"));
    assert!(output.contains("**Hermes:** enabled"));
    assert!(output.contains("**Codegen config:** missing"));
}

#[test]
fn react_native_section_absent_when_profile_is_none() {
    let summary = ScanSummary {
        root_path: PathBuf::from("web-project"),
        react_native: None,
        ..ScanSummary::default()
    };

    let output = render_scan_summary(&summary, OutputFormat::Markdown)
        .expect("failed to render markdown summary");

    assert!(!output.contains("### React Native"));
}
