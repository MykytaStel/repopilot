use repopilot::scan::scanner::scan_path;
use std::fs;
use tempfile::tempdir;

#[test]
fn scan_adds_large_file_architecture_finding() {
    let temp = tempdir().expect("failed to create temp dir");
    let file_path = temp.path().join("large.rs");

    let content = (0..301)
        .map(|index| format!("fn function_{index}() {{}}"))
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(&file_path, content).expect("failed to write large file");

    let summary = scan_path(temp.path()).expect("failed to scan temp project");

    let finding = summary
        .findings
        .iter()
        .find(|finding| finding.rule_id == "architecture.large-file")
        .expect("expected large file finding");

    assert_eq!(finding.title, "Large file detected");
    assert_eq!(finding.evidence.len(), 1);
    assert_eq!(finding.evidence[0].path, file_path);
    assert!(finding.evidence[0].snippet.contains("301"));
}
