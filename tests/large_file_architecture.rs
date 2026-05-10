use repopilot::audits::architecture::large_file::{LargeFileAudit, detect_large_file_finding};
use repopilot::audits::traits::FileAudit;
use repopilot::findings::types::{FindingCategory, Severity};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::facts::FileFacts;
use std::path::Path;
use std::path::PathBuf;

#[test]
fn does_not_create_finding_when_file_is_under_threshold() {
    let config = ScanConfig::default();

    let finding = detect_large_file_finding(
        Path::new("src/small.rs"),
        config.large_file_loc_threshold,
        &config,
    );

    assert!(finding.is_none());
}

#[test]
fn creates_architecture_finding_when_file_is_above_threshold() {
    let config = ScanConfig::default();
    let lines_of_code = config.large_file_loc_threshold + 1;

    let finding = detect_large_file_finding(Path::new("src/large.rs"), lines_of_code, &config)
        .expect("expected large file finding");

    assert_eq!(finding.rule_id, "architecture.large-file");
    assert_eq!(finding.category, FindingCategory::Architecture);
    assert_eq!(finding.severity, Severity::Medium);
    assert_eq!(finding.evidence.len(), 1);

    let evidence = &finding.evidence[0];

    assert_eq!(evidence.path, Path::new("src/large.rs"));
    assert_eq!(evidence.line_start, 1);
    assert!(evidence.snippet.contains("301"));
    assert!(evidence.snippet.contains("configured threshold"));
}

#[test]
fn uses_high_severity_for_very_large_files() {
    let config = ScanConfig::default();

    let finding = detect_large_file_finding(
        Path::new("src/huge.rs"),
        config.huge_file_loc_threshold,
        &config,
    )
    .expect("expected huge file finding");

    assert_eq!(finding.severity, Severity::High);
}

#[test]
fn respects_custom_large_file_threshold() {
    let config = ScanConfig::default().with_large_file_loc_threshold(500);

    let below_custom_threshold =
        detect_large_file_finding(Path::new("src/not_large.rs"), 400, &config);

    let above_custom_threshold = detect_large_file_finding(Path::new("src/large.rs"), 501, &config);

    assert!(below_custom_threshold.is_none());
    assert!(above_custom_threshold.is_some());
}

#[test]
fn large_file_audit_skips_non_code_files() {
    for (path, language) in [
        ("Cargo.lock", Some("TOML")),
        ("pnpm-lock.yaml", Some("YAML")),
        ("README.md", Some("Markdown")),
        ("data.json", Some("JSON")),
    ] {
        let file = FileFacts {
            path: PathBuf::from(path),
            language: language.map(str::to_string),
            lines_of_code: 5_000,
            branch_count: 0,
            imports: Vec::new(),
            content: None,
            has_inline_tests: false,
        };

        let findings = LargeFileAudit.audit(&file, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "non-code file must not be flagged as large source: {path}"
        );
    }
}

#[test]
fn large_file_audit_skips_test_and_fixture_paths() {
    for path in ["tests/large_scan.rs", "src/fixtures/generated.rs"] {
        let file = FileFacts {
            path: PathBuf::from(path),
            language: Some("Rust".to_string()),
            lines_of_code: 5_000,
            branch_count: 0,
            imports: Vec::new(),
            content: None,
            has_inline_tests: false,
        };

        let findings = LargeFileAudit.audit(&file, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "test and fixture paths must not be flagged as large source: {path}"
        );
    }
}
