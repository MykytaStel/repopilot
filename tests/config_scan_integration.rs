use repopilot::config::loader::parse_config;
use repopilot::scan::scanner::scan_path_with_config;
use std::fs;
use tempfile::tempdir;

#[test]
fn ignore_list_from_config_is_applied() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::create_dir_all(temp.path().join("generated")).expect("failed to create generated dir");
    fs::create_dir_all(temp.path().join("src")).expect("failed to create src dir");
    fs::write(
        temp.path().join("generated/ignored.rs"),
        "fn ignored() {}\n",
    )
    .expect("failed to write ignored file");
    fs::write(temp.path().join("src/lib.rs"), "pub fn live() {}\n")
        .expect("failed to write src file");

    let repo_config = parse_config(
        r#"
        [scan]
        ignore = ["generated"]
        "#,
        None,
    )
    .expect("valid config should parse");

    let summary = scan_path_with_config(temp.path(), &repo_config.to_scan_config())
        .expect("failed to scan temp project");

    assert_eq!(summary.files_count, 1);
}

#[test]
fn custom_max_file_lines_from_config_affects_large_file_findings() {
    let temp = tempdir().expect("failed to create temp dir");
    let file_path = temp.path().join("small.rs");

    let content = (0..11)
        .map(|index| format!("fn function_{index}() {{}}"))
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(file_path, content).expect("failed to write file");

    let repo_config = parse_config(
        r#"
        [architecture]
        max_file_lines = 10
        "#,
        None,
    )
    .expect("valid config should parse");

    let summary = scan_path_with_config(temp.path(), &repo_config.to_scan_config())
        .expect("failed to scan temp project");

    assert!(
        summary
            .findings
            .iter()
            .any(|finding| finding.rule_id == "architecture.large-file")
    );
}
