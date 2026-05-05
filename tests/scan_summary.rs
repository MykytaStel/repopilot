use repopilot::scan::scanner::scan_path;
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
    assert_eq!(summary.findings.len(), 1);
    assert_eq!(summary.findings[0].rule_id, "code-marker.todo");
    assert_eq!(summary.findings[0].evidence[0].line_start, 3);

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
