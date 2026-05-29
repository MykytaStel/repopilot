use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::{collect_scan_facts_with_config, scan_path};
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

    assert_eq!(summary.metrics.files_analyzed, 1);
    assert!(summary.artifacts.findings.iter().all(|finding| {
        finding
            .evidence
            .iter()
            .all(|evidence| !evidence.path.to_string_lossy().contains(".git"))
    }));
}

#[test]
fn scanner_exclude_patterns_support_globs_and_file_names() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::create_dir_all(temp.path().join("src/generated")).expect("failed to create generated dir");
    fs::create_dir_all(temp.path().join("fixtures")).expect("failed to create fixtures dir");

    fs::write(temp.path().join("src/lib.rs"), "pub fn live() {}\n").expect("write live");
    fs::write(
        temp.path().join("src/generated/client.rs"),
        "pub fn generated() {}\n",
    )
    .expect("write generated");
    fs::write(
        temp.path().join("fixtures/data.rs"),
        "pub fn fixture() {}\n",
    )
    .expect("write fixture");
    fs::write(temp.path().join("ignored.snap"), "snapshot\n").expect("write snap");

    let facts = collect_scan_facts_with_config(
        temp.path(),
        &ScanConfig {
            include_low_signal: true,
            exclude_patterns: vec![
                "src/generated/**".to_string(),
                "fixtures".to_string(),
                "*.snap".to_string(),
            ],
            ..ScanConfig::default()
        },
    )
    .expect("failed to scan");

    assert_eq!(facts.files_discovered, 1);
    assert_eq!(facts.files_analyzed, 1);
    assert_eq!(facts.files[0].path, temp.path().join("src/lib.rs"));
}

#[test]
fn repopilotignore_accounting_uses_single_filtered_walk_for_files() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join(".repopilotignore"), "ignored.rs\n*.snap\n").expect("write ignore");
    fs::write(temp.path().join("kept.rs"), "fn kept() {}\n").expect("write kept");
    fs::write(temp.path().join("ignored.rs"), "fn ignored() {}\n").expect("write ignored");
    fs::write(temp.path().join("state.snap"), "snapshot\n").expect("write snap");

    let summary = scan_path(temp.path()).expect("failed to scan");

    assert_eq!(summary.metrics.files_discovered, 1);
    assert_eq!(summary.metrics.files_analyzed, 1);
    assert_eq!(summary.metrics.files_skipped_repopilotignore, 2);
}
