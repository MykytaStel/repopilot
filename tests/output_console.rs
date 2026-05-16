use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::scan::types::ScanSummary;
use std::path::PathBuf;

#[test]
fn console_output_includes_versioned_summary_and_grouped_findings() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        files_discovered: 0,
        files_count: 1,
        directories_count: 1,
        lines_of_code: 100,
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
        files_count: 1,
        directories_count: 1,
        lines_of_code: 10,
        health_score: 100,
        files_skipped_by_limit: 2,
        ..ScanSummary::default()
    };

    let output = render_scan_summary(&summary, OutputFormat::Console)
        .expect("failed to render console report");

    assert!(output.contains("Files skipped (limit):"));
    assert!(!output.contains("Files skipped (ignore):"));
}
