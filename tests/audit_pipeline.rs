use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use std::fs;
use tempfile::tempdir;

#[test]
fn scan_pipeline_produces_file_audit_findings() {
    let temp = tempdir().expect("failed to create temp dir");
    let src = temp.path().join("src");
    fs::create_dir_all(&src).unwrap();

    let content = (0..301)
        .map(|index| format!("fn function_{index}() {{}}"))
        .chain(std::iter::once("// TODO: split this file".to_string()))
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(src.join("large.rs"), content).expect("failed to write file");

    let summary =
        scan_path_with_config(temp.path(), &ScanConfig::default()).expect("scan failed");

    assert!(
        summary
            .findings
            .iter()
            .any(|f| f.rule_id == "architecture.large-file"),
        "expected large-file finding"
    );

    assert!(
        summary
            .findings
            .iter()
            .any(|f| f.rule_id == "code-marker.todo"),
        "expected todo marker finding"
    );
}
