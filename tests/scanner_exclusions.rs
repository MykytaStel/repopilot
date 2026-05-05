use repopilot::scan::scanner::scan_path;
use std::fs;
use tempfile::tempdir;

#[test]
fn scanner_ignores_git_and_build_directories() {
    let temp = tempdir().expect("failed to create temp dir");

    fs::create_dir_all(temp.path().join(".git/hooks")).expect("failed to create git hooks");
    fs::create_dir_all(temp.path().join("target/debug")).expect("failed to create target dir");
    fs::create_dir_all(temp.path().join("src")).expect("failed to create src dir");

    fs::write(
        temp.path().join(".git/hooks/pre-commit.sample"),
        "# TODO: this git hook must not be scanned\n",
    )
    .expect("failed to write hook");
    fs::write(
        temp.path().join("target/debug/generated.rs"),
        "// TODO: generated file must not be scanned\n",
    )
    .expect("failed to write generated file");
    fs::write(temp.path().join("src/lib.rs"), "pub fn live() {}\n").expect("failed to write src");

    let summary = scan_path(temp.path()).expect("failed to scan");

    assert_eq!(summary.files_count, 1);
    assert!(summary.findings.iter().all(|finding| {
        finding
            .evidence
            .iter()
            .all(|evidence| !evidence.path.to_string_lossy().contains(".git"))
    }));
}
