use repopilot::baseline::diff::{BaselineScanReport, BaselineStatus, FindingBaselineStatus};
use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::output::{OutputFormat, render_baseline_scan_report, render_scan_summary};
use repopilot::scan::types::{
    ChangedFileCacheTelemetry, ChangedFileReasonSummary, HiddenSuggestionSummary, ScanArtifacts,
    ScanCacheTelemetry, ScanCacheTimings, ScanMetadata, ScanMetrics, ScanSummary,
};
use std::path::PathBuf;

#[test]
fn console_output_includes_versioned_summary_and_grouped_findings() {
    let summary = ScanSummary {
        metadata: ScanMetadata {
            root_path: PathBuf::from("demo"),
            ..Default::default()
        },
        metrics: ScanMetrics {
            files_analyzed: 1,
            directories_count: 1,
            non_empty_lines: 100,
            health_score: 95,
            ..Default::default()
        },
        artifacts: ScanArtifacts {
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
                provenance: Default::default(),
                risk: Default::default(),
            }],
            ..Default::default()
        },
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
        metadata: ScanMetadata {
            root_path: PathBuf::from("demo"),
            ..Default::default()
        },
        metrics: ScanMetrics {
            files_discovered: 3,
            files_analyzed: 1,
            directories_count: 1,
            non_empty_lines: 10,
            health_score: 100,
            files_skipped_by_limit: 2,
            ..Default::default()
        },
        ..Default::default()
    };

    let output = render_scan_summary(&summary, OutputFormat::Console)
        .expect("failed to render console report");

    assert!(output.contains("Files skipped (limit):"));
    assert!(!output.contains("Files skipped (ignore):"));
}

#[test]
fn console_output_includes_hidden_suggestion_breakdown() {
    let summary = ScanSummary {
        metadata: ScanMetadata {
            root_path: PathBuf::from("demo"),
            ..Default::default()
        },
        metrics: ScanMetrics {
            hidden_suggestions_count: 3,
            ..Default::default()
        },
        artifacts: ScanArtifacts {
            hidden_suggestions: vec![HiddenSuggestionSummary {
                intent: "maintainability".to_string(),
                rule_id: "architecture.large-file".to_string(),
                category: "architecture".to_string(),
                reason: "maintainability signals are hidden in the default profile".to_string(),
                count: 3,
            }],
            ..Default::default()
        },
    };

    let output = render_scan_summary(&summary, OutputFormat::Console)
        .expect("failed to render console report");

    assert!(output.contains("Hidden suggestions breakdown:"));
    assert!(output.contains("architecture / maintainability / architecture.large-file"));
}

#[test]
fn console_output_includes_cache_telemetry_for_changed_scans() {
    let summary = ScanSummary {
        metadata: ScanMetadata {
            root_path: PathBuf::from("demo"),
            cache_telemetry: Some(cache_telemetry()),
            ..Default::default()
        },
        ..Default::default()
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

#[test]
fn console_baseline_status_stays_aligned_for_duplicate_findings() {
    let report = duplicate_status_report();

    let output = render_baseline_scan_report(&report, OutputFormat::Console, None)
        .expect("failed to render baseline console");

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
            metadata: ScanMetadata {
                root_path: PathBuf::from("demo"),
                ..Default::default()
            },
            artifacts: ScanArtifacts {
                findings: vec![
                    duplicate_finding("existing", 7),
                    duplicate_finding("new", 8),
                ],
                ..Default::default()
            },
            ..Default::default()
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
