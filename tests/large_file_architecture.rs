use repopilot::audits::architecture::large_file::{
    LARGE_FILE_LOC_THRESHOLD, detect_large_file_finding,
};
use repopilot::findings::types::{FindingCategory, Severity};
use std::path::Path;

#[test]
fn does_not_create_finding_when_file_is_under_threshold() {
    let finding = detect_large_file_finding(Path::new("src/small.rs"), LARGE_FILE_LOC_THRESHOLD);

    assert!(finding.is_none());
}

#[test]
fn creates_architecture_finding_when_file_is_above_threshold() {
    let lines_of_code = LARGE_FILE_LOC_THRESHOLD + 1;

    let finding = detect_large_file_finding(Path::new("src/large.rs"), lines_of_code)
        .expect("expected large file finding");

    assert_eq!(finding.rule_id, "architecture.large-file");
    assert_eq!(finding.category, FindingCategory::Architecture);
    assert_eq!(finding.severity, Severity::Medium);
    assert_eq!(finding.evidence.len(), 1);

    let evidence = &finding.evidence[0];

    assert_eq!(evidence.path, Path::new("src/large.rs"));
    assert_eq!(evidence.line_start, 1);
    assert!(evidence.snippet.contains("301"));
    assert!(evidence.snippet.contains("threshold"));
}

#[test]
fn uses_high_severity_for_very_large_files() {
    let finding = detect_large_file_finding(Path::new("src/huge.rs"), 1000)
        .expect("expected huge file finding");

    assert_eq!(finding.severity, Severity::High);
}
