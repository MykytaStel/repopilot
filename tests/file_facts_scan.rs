use repopilot::scan::scanner::collect_scan_facts;
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
    assert!(file.content.contains("TODO"));
}
