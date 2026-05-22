use repopilot::baseline::diff::{BaselineScanReport, BaselineStatus, FindingBaselineStatus};
use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::frameworks::ReactNativeArchitectureProfile;
use repopilot::output::{OutputFormat, render_baseline_scan_report, render_scan_summary};
use repopilot::scan::types::{
    ChangedFileCacheTelemetry, ChangedFileReasonSummary, HiddenSuggestionSummary, LanguageSummary,
    ScanCacheTelemetry, ScanCacheTimings, ScanSummary,
};
use std::path::PathBuf;

#[test]
fn renders_markdown_scan_summary() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo-project"),
        mode: Default::default(),
        base_ref: None,
        changed_files_count: 0,
        repo_level_rules_included: true,
        files_discovered: 0,
        files_analyzed: 2,
        directories_count: 1,
        non_empty_lines: 10,
        large_files_skipped: 0,
        files_skipped_low_signal: 0,
        binary_files_skipped: 0,
        skipped_bytes: 0,
        languages: vec![
            LanguageSummary {
                name: "Rust".to_string(),
                files_analyzed: 1,
            },
            LanguageSummary {
                name: "TypeScript".to_string(),
                files_analyzed: 1,
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
            provenance: Default::default(),
            risk: Default::default(),
        }],
        detected_frameworks: vec![],
        framework_projects: vec![],
        react_native: None,
        coupling_graph: None,
        context_graph_summary: None,
        context_graph_cache: None,
        scan_duration_us: 0,
        health_score: 0,
        raw_findings_count: 1,
        visible_findings_count: 1,
        hidden_suggestions_count: 0,
        hidden_suggestions: Vec::new(),
        visibility_profile: None,
        files_skipped_by_limit: 0,
        files_skipped_repopilotignore: 0,
        repopilotignore_path: None,
        scan_timings: None,
        cache_telemetry: None,
        local_feedback: None,
        diagnostics: Vec::new(),
        raw_signal_quality: Default::default(),
        visible_signal_quality: Default::default(),
        signal_quality: Default::default(),
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
    assert!(output.contains("## Top Risk Clusters"));
    assert!(output.contains("## Top Rules"));
    assert!(output.contains("## Languages"));
    assert!(output.contains("## Findings Index"));
    assert!(output.contains("## Findings"));
    assert!(output.contains("### Code Quality"));
    assert!(output.contains("#### `code-marker.todo` (1)"));
    assert!(output.contains("| `code-marker.todo` | 1 | LOW |"));
    assert!(output.contains("| P3 | `src` | `code-marker.todo` | 1 | LOW | 0 |"));
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
        mode: Default::default(),
        base_ref: None,
        changed_files_count: 0,
        repo_level_rules_included: true,
        files_discovered: 0,
        files_analyzed: 0,
        directories_count: 0,
        non_empty_lines: 0,
        large_files_skipped: 0,
        files_skipped_low_signal: 0,
        binary_files_skipped: 0,
        skipped_bytes: 0,
        languages: vec![],
        findings: vec![],
        detected_frameworks: vec![],
        framework_projects: vec![],
        react_native: None,
        coupling_graph: None,
        context_graph_summary: None,
        context_graph_cache: None,
        scan_duration_us: 0,
        health_score: 0,
        raw_findings_count: 0,
        visible_findings_count: 0,
        hidden_suggestions_count: 0,
        hidden_suggestions: Vec::new(),
        visibility_profile: None,
        files_skipped_by_limit: 0,
        files_skipped_repopilotignore: 0,
        repopilotignore_path: None,
        scan_timings: None,
        cache_telemetry: None,
        local_feedback: None,
        diagnostics: Vec::new(),
        raw_signal_quality: Default::default(),
        visible_signal_quality: Default::default(),
        signal_quality: Default::default(),
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
        mode: Default::default(),
        base_ref: None,
        changed_files_count: 0,
        repo_level_rules_included: true,
        files_discovered: 0,
        files_analyzed: 5,
        directories_count: 2,
        non_empty_lines: 100,
        large_files_skipped: 0,
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
        context_graph_summary: None,
        context_graph_cache: None,
        scan_duration_us: 0,
        health_score: 0,
        raw_findings_count: 0,
        visible_findings_count: 0,
        hidden_suggestions_count: 0,
        hidden_suggestions: Vec::new(),
        visibility_profile: None,
        files_skipped_by_limit: 0,
        files_skipped_repopilotignore: 0,
        repopilotignore_path: None,
        scan_timings: None,
        cache_telemetry: None,
        local_feedback: None,
        diagnostics: Vec::new(),
        raw_signal_quality: Default::default(),
        visible_signal_quality: Default::default(),
        signal_quality: Default::default(),
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

#[test]
fn markdown_output_includes_hidden_suggestion_breakdown() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        hidden_suggestions_count: 2,
        hidden_suggestions: vec![HiddenSuggestionSummary {
            intent: "testing-gap".to_string(),
            rule_id: "testing.source-without-test".to_string(),
            category: "testing".to_string(),
            reason: "testing gaps are hidden in the default profile".to_string(),
            count: 2,
        }],
        ..ScanSummary::default()
    };

    let output = render_scan_summary(&summary, OutputFormat::Markdown)
        .expect("failed to render markdown summary");

    assert!(output.contains("- **Top hidden suggestions:**"));
    assert!(output.contains("`testing` / `testing-gap` / `testing.source-without-test`: 2"));
}

#[test]
fn markdown_output_includes_cache_telemetry() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        cache_telemetry: Some(cache_telemetry()),
        ..ScanSummary::default()
    };

    let output = render_scan_summary(&summary, OutputFormat::Markdown)
        .expect("failed to render markdown summary");

    assert!(output.contains("- **Cache:** 1 hit(s), 1 miss(es), 0 skipped (50% hit rate)"));
    assert!(output.contains("- **Changed file reasons:** modified (2)"));
    assert!(output.contains("- **Cache timing:** load 1ms; hash 2ms; lookup 3ms; reuse 4ms; miss scan 5ms; write 6ms; est. saved 7ms"));
    assert!(output.contains("`src/lib.rs`: hit (modified, unchanged-content-and-config)"));
}

#[test]
fn markdown_baseline_status_stays_aligned_for_duplicate_findings() {
    let report = duplicate_status_report();

    let output = render_baseline_scan_report(&report, OutputFormat::Markdown, None)
        .expect("failed to render baseline markdown");

    let existing = output
        .find("Baseline: existing")
        .expect("existing status should render");
    let new = output
        .find("Baseline: new")
        .expect("new status should render");
    assert!(existing < new);
}

fn cache_telemetry() -> ScanCacheTelemetry {
    ScanCacheTelemetry {
        hits: 1,
        misses: 1,
        skipped: 0,
        hit_rate_percent: 50,
        changed_file_reasons: vec![ChangedFileReasonSummary {
            reason: "modified".to_string(),
            count: 2,
        }],
        changed_files: vec![ChangedFileCacheTelemetry {
            path: PathBuf::from("src/lib.rs"),
            change_reason: "modified".to_string(),
            cache_status: "hit".to_string(),
            cache_reason: "unchanged-content-and-config".to_string(),
        }],
        timings: ScanCacheTimings {
            load_us: 1_000,
            file_hash_us: 2_000,
            lookup_us: 3_000,
            hit_reuse_us: 4_000,
            miss_scan_us: 5_000,
            write_us: 6_000,
            estimated_time_saved_us: Some(7_000),
        },
    }
}

fn duplicate_status_report() -> BaselineScanReport {
    BaselineScanReport {
        summary: ScanSummary {
            root_path: PathBuf::from("demo"),
            findings: vec![
                duplicate_finding("existing", 7),
                duplicate_finding("new", 8),
            ],
            ..ScanSummary::default()
        },
        baseline_path: None,
        findings: vec![
            FindingBaselineStatus {
                key: "existing".to_string(),
                status: BaselineStatus::Existing,
            },
            FindingBaselineStatus {
                key: "new".to_string(),
                status: BaselineStatus::New,
            },
        ],
    }
}

fn duplicate_finding(id: &str, line: usize) -> Finding {
    Finding {
        id: id.to_string(),
        rule_id: "code-quality.long-function".to_string(),
        recommendation: Finding::recommendation_for_rule_id("code-quality.long-function"),
        title: "Duplicate-looking finding".to_string(),
        description: "Duplicate-looking finding for renderer status alignment.".to_string(),
        category: FindingCategory::CodeQuality,
        severity: Severity::Medium,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from("src/lib.rs"),
            line_start: line,
            line_end: None,
            snippet: "fn duplicate() {}".to_string(),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}
