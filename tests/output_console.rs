use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::scan::types::{
    ChangedFileCacheTelemetry, ChangedFileReasonSummary, HiddenSuggestionSummary,
    ScanCacheTelemetry, ScanCacheTimings, ScanSummary,
};
use std::path::PathBuf;

#[test]
fn console_output_includes_versioned_summary_and_grouped_findings() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        files_discovered: 0,
        files_analyzed: 1,
        directories_count: 1,
        non_empty_lines: 100,
        findings: vec![Finding {
            id: "finding-1".to_string(),
            rule_id: "security.secret-candidate".to_string(),
            recommendation: Finding::recommendation_for_rule_id("security.secret-candidate"),
            title: "Possible secret detected".to_string(),
            description: "A real secret may be committed.".to_string(),
            category: FindingCategory::Security,
            severity: Severity::High,
            confidence: Default::default(),
            evidence: vec![Evidence {
                path: PathBuf::from("src/config.rs"),
                line_start: 7,
                line_end: None,
                snippet: "API_KEY = \"abc123xyz987\"".to_string(),
            }],
            workspace_package: None,
            docs_url: None,
            risk: Default::default(),
        }],
        health_score: 95,
        ..ScanSummary::default()
    };

    let output = render_scan_summary(&summary, OutputFormat::Console)
        .expect("failed to render console report");

    assert!(output.contains("RepoPilot Scan"));
    assert!(output.contains(&format!("Version: {}", env!("CARGO_PKG_VERSION"))));
    assert!(output.contains("Risk Summary:"));
    assert!(output.contains("Top Risk Clusters:"));
    assert!(output.contains("Top Rules:"));
    assert!(output.contains("Findings:"));
    assert!(output.contains("Security:"));
    assert!(output.contains("security.secret-candidate (1)"));
    assert!(output.contains("src/config.rs:7"));
    assert!(output.contains("Recommendation:"));
}

#[test]
fn console_output_labels_max_files_remainder_as_limit_skipped() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        files_discovered: 3,
        files_analyzed: 1,
        directories_count: 1,
        non_empty_lines: 10,
        health_score: 100,
        files_skipped_by_limit: 2,
        ..ScanSummary::default()
    };

    let output = render_scan_summary(&summary, OutputFormat::Console)
        .expect("failed to render console report");

    assert!(output.contains("Files skipped (limit):"));
    assert!(!output.contains("Files skipped (ignore):"));
}

#[test]
fn console_output_includes_hidden_suggestion_breakdown() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        hidden_suggestions_count: 3,
        hidden_suggestions: vec![HiddenSuggestionSummary {
            intent: "maintainability".to_string(),
            rule_id: "architecture.large-file".to_string(),
            category: "architecture".to_string(),
            reason: "maintainability signals are hidden in the default profile".to_string(),
            count: 3,
        }],
        ..ScanSummary::default()
    };

    let output = render_scan_summary(&summary, OutputFormat::Console)
        .expect("failed to render console report");

    assert!(output.contains("Hidden suggestions breakdown:"));
    assert!(output.contains("architecture / maintainability / architecture.large-file"));
}

#[test]
fn console_output_includes_cache_telemetry_for_changed_scans() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        cache_telemetry: Some(cache_telemetry()),
        ..ScanSummary::default()
    };

    let output = render_scan_summary(&summary, OutputFormat::Console)
        .expect("failed to render console report");

    assert!(output.contains("Cache telemetry:"));
    assert!(output.contains("Cache hits:"));
    assert!(output.contains("misses:"));
    assert!(output.contains("Changed file reasons: modified (2)"));
    assert!(output.contains("Cache timing:"));
    assert!(output.contains("src/lib.rs: hit (modified, unchanged-content-and-config)"));
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
