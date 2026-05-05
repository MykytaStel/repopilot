use repopilot::audits::testing::source_without_test::SourceWithoutTestAudit;
use repopilot::audits::traits::ProjectAudit;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::facts::{FileFacts, ScanFacts};
use std::path::PathBuf;

#[test]
fn reports_missing_test_folder() {
    let facts = ScanFacts {
        root_path: PathBuf::from("demo"),
        files: vec![file("src/payment.rs")],
        files_count: 1,
        ..ScanFacts::default()
    };

    let findings = repopilot::audits::testing::missing_test_folder::MissingTestFolderAudit
        .audit(&facts, &ScanConfig::default());

    assert_eq!(findings[0].rule_id, "testing.missing-test-folder");
}

#[test]
fn source_without_test_recognizes_integration_test_counterpart() {
    let facts = ScanFacts {
        root_path: PathBuf::from("demo"),
        files: vec![file("src/report/writer.rs"), file("tests/report_writer.rs")],
        files_count: 2,
        ..ScanFacts::default()
    };

    let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn source_without_test_reports_uncovered_source_and_ignores_wrappers() {
    let facts = ScanFacts {
        root_path: PathBuf::from("demo"),
        files: vec![
            file("src/payment.rs"),
            file("src/lib.rs"),
            file("src/mod.rs"),
        ],
        files_count: 3,
        ..ScanFacts::default()
    };

    let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].evidence[0].path,
        PathBuf::from("src/payment.rs")
    );
}

fn file(path: &str) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: Some("Rust".to_string()),
        lines_of_code: 1,
        content: "pub fn value() {}\n".to_string(),
    }
}
