use repopilot::findings::types::{Finding, FindingCategory, Severity};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path;
use repopilot::scan::scanner::scan_path_with_config;
use repopilot::scan::types::ScanSummary;
use std::fs;
use tempfile::tempdir;

#[test]
fn scans_directory_with_counts_languages_and_markers() {
    let temp = tempdir().expect("failed to create temp dir");
    let src_dir = temp.path().join("src");

    fs::create_dir(&src_dir).expect("failed to create src dir");

    fs::write(
        src_dir.join("main.rs"),
        "fn main() {}\n\n// TODO: add scanner test\n",
    )
    .expect("failed to write rust file");

    fs::write(src_dir.join("app.ts"), "const value = 1;\n").expect("failed to write ts file");

    fs::write(temp.path().join("README.md"), "# RepoPilot\n")
        .expect("failed to write markdown file");

    let summary = scan_path(temp.path()).expect("failed to scan temp project");

    assert_eq!(summary.directories_count, 1);
    assert_eq!(summary.files_count, 3);
    assert_eq!(summary.lines_of_code, 4);

    let todo_finding = summary
        .findings
        .iter()
        .find(|f| f.rule_id == "code-marker.todo")
        .expect("expected a code-marker.todo finding");
    assert_eq!(todo_finding.evidence[0].line_start, 3);

    assert!(
        summary
            .languages
            .iter()
            .any(|language| language.name == "Rust" && language.files_count == 1)
    );

    assert!(
        summary
            .languages
            .iter()
            .any(|language| language.name == "TypeScript" && language.files_count == 1)
    );

    assert!(
        summary
            .languages
            .iter()
            .any(|language| language.name == "Markdown" && language.files_count == 1)
    );
}

#[test]
fn scan_reports_files_skipped_by_size_guard() {
    let temp = tempdir().expect("failed to create temp dir");
    let file_path = temp.path().join("large.rs");
    let content = "fn large() {}\n".repeat(20);
    fs::write(&file_path, &content).expect("failed to write large file");

    let config = ScanConfig {
        max_file_bytes: 16,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan temp project");

    assert_eq!(summary.files_count, 0);
    assert_eq!(summary.files_discovered, 1);
    assert_eq!(summary.skipped_files_count, 1);
    assert_eq!(summary.skipped_bytes, content.len() as u64);
    assert_eq!(summary.lines_of_code, 0);
}

#[test]
fn scan_reports_binary_files_as_skipped_without_failing() {
    let temp = tempdir().expect("failed to create temp dir");
    let bytes = [0xff, 0xfe, 0xfd, 0x00, 0x61];
    fs::write(temp.path().join("binary.rs"), bytes).expect("failed to write binary file");

    let summary = scan_path(temp.path()).expect("failed to scan temp project");

    assert_eq!(summary.files_count, 0);
    assert_eq!(summary.files_discovered, 1);
    assert_eq!(summary.skipped_files_count, 0);
    assert_eq!(summary.binary_files_skipped, 1);
    assert_eq!(summary.skipped_bytes, bytes.len() as u64);
    assert_eq!(summary.lines_of_code, 0);
}

#[test]
fn health_score_is_100_with_no_findings() {
    assert_eq!(ScanSummary::compute_health_score(&[], 1000), 100);
}

#[test]
fn health_score_decreases_with_severity() {
    fn finding(severity: Severity) -> Finding {
        Finding {
            id: String::new(),
            rule_id: "test".to_string(),
            recommendation: Finding::recommendation_for_rule_id("test"),
            title: String::new(),
            description: String::new(),
            category: FindingCategory::Security,
            severity,
            confidence: Default::default(),
            evidence: vec![],
            workspace_package: None,
            docs_url: None,
        }
    }
    let critical = ScanSummary::compute_health_score(&[finding(Severity::Critical)], 1000);
    let high = ScanSummary::compute_health_score(&[finding(Severity::High)], 1000);
    let medium = ScanSummary::compute_health_score(&[finding(Severity::Medium)], 1000);
    let low = ScanSummary::compute_health_score(&[finding(Severity::Low)], 1000);

    assert!(critical < high, "critical should score worse than high");
    assert!(high < medium, "high should score worse than medium");
    assert!(medium < low, "medium should score worse than low");
    assert!(low < 100, "any finding should reduce score from 100");
}

#[test]
fn health_score_is_clamped_at_zero_for_catastrophic_repos() {
    let findings: Vec<Finding> = (0..10)
        .map(|_| Finding {
            id: String::new(),
            rule_id: "test".to_string(),
            recommendation: Finding::recommendation_for_rule_id("test"),
            title: String::new(),
            description: String::new(),
            category: FindingCategory::Security,
            severity: Severity::Critical,
            confidence: Default::default(),
            evidence: vec![],
            workspace_package: None,
            docs_url: None,
        })
        .collect();
    assert_eq!(
        ScanSummary::compute_health_score(&findings, 100),
        0,
        "score must clamp to 0"
    );
}

#[test]
fn scan_result_includes_health_score() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("main.rs"), "fn main() {}\n").expect("write file");

    let summary = scan_path(temp.path()).expect("scan");

    assert!(
        summary.health_score > 0,
        "clean project should have positive health score"
    );
}
