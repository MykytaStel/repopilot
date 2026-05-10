use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::collect_scan_facts;
use repopilot::scan::scanner::collect_scan_facts_with_config;
use std::fs;
use tempfile::tempdir;

#[test]
fn scanner_collects_file_facts_without_running_rules_directly() {
    let temp = tempdir().expect("failed to create temp dir");
    let file_path = temp.path().join("main.rs");

    fs::write(
        &file_path,
        "fn main() {}\n\n// TODO: this is only a fact at scan stage\n",
    )
    .expect("failed to write file");

    let facts = collect_scan_facts(temp.path()).expect("failed to collect scan facts");

    assert_eq!(facts.files_count, 1);
    assert_eq!(facts.lines_of_code, 2);
    assert_eq!(facts.files.len(), 1);

    let file = &facts.files[0];

    assert_eq!(file.path, file_path);
    assert_eq!(file.language.as_deref(), Some("Rust"));
    assert_eq!(file.lines_of_code, 2);
    assert!(file.content.as_deref().unwrap_or("").contains("TODO"));
}

#[test]
fn collector_reports_files_skipped_by_size_guard() {
    let temp = tempdir().expect("failed to create temp dir");
    let file_path = temp.path().join("large.rs");
    let content = "pub fn large() {}\n".repeat(20);
    fs::write(&file_path, &content).expect("failed to write large file");

    let config = ScanConfig {
        max_file_bytes: 16,
        ..ScanConfig::default()
    };

    let facts =
        collect_scan_facts_with_config(temp.path(), &config).expect("failed to collect scan facts");

    assert_eq!(facts.files_count, 0);
    assert_eq!(facts.skipped_files_count, 1);
    assert_eq!(facts.skipped_bytes, content.len() as u64);
    assert_eq!(facts.files.len(), 1);
    assert!(facts.files[0].content.is_none());
}
