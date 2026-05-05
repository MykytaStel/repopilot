use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use std::fs;
use tempfile::tempdir;

#[test]
fn scan_respects_custom_large_file_threshold() {
    let temp = tempdir().expect("failed to create temp dir");
    let file_path = temp.path().join("medium.rs");

    let content = (0..301)
        .map(|index| format!("fn function_{index}() {{}}"))
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(file_path, content).expect("failed to write file");

    let config = ScanConfig::default().with_large_file_loc_threshold(500);
    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan temp project");

    assert!(
        summary
            .findings
            .iter()
            .all(|finding| finding.rule_id != "architecture.large-file")
    );
}

#[test]
fn scan_still_reports_file_above_custom_threshold() {
    let temp = tempdir().expect("failed to create temp dir");
    let file_path = temp.path().join("large.rs");

    let content = (0..501)
        .map(|index| format!("fn function_{index}() {{}}"))
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(file_path, content).expect("failed to write file");

    let config = ScanConfig::default().with_large_file_loc_threshold(500);
    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan temp project");

    let finding = summary
        .findings
        .iter()
        .find(|finding| finding.rule_id == "architecture.large-file")
        .expect("expected large file finding");

    assert!(finding.evidence[0].snippet.contains("501"));
    assert!(finding.evidence[0].snippet.contains("500"));
}
