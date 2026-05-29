use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use std::fs;

#[test]
fn scan_counts_files_skipped_by_repopilotignore() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();

    fs::write(root.join(".repopilotignore"), "ignored.rs\n").expect("write ignore");
    fs::write(root.join("kept.rs"), "fn main() {}\n").expect("write kept file");
    fs::write(root.join("ignored.rs"), "fn ignored() {}\n").expect("write ignored file");

    let summary = scan_path_with_config(root, &ScanConfig::default()).expect("scan");

    assert_eq!(summary.metrics.files_discovered, 1);
    assert_eq!(summary.metrics.files_skipped_repopilotignore, 1);
    assert_eq!(summary.metrics.files_analyzed, 1);
    assert!(summary.repopilotignore_path.is_some());
}

#[test]
fn scan_counts_files_skipped_by_max_files_limit() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();

    fs::write(root.join("a.rs"), "fn a() {}\n").expect("write a");
    fs::write(root.join("b.rs"), "fn b() {}\n").expect("write b");
    fs::write(root.join("c.rs"), "fn c() {}\n").expect("write c");

    let config = ScanConfig {
        max_files: Some(1),
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(root, &config).expect("scan");

    assert_eq!(summary.metrics.files_discovered, 3);
    assert_eq!(summary.metrics.files_skipped_by_limit, 2);
    assert_eq!(summary.metrics.files_analyzed, 1);
}

#[test]
fn scan_does_not_report_repopilotignore_when_file_is_missing() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();

    fs::write(root.join("main.rs"), "fn main() {}\n").expect("write file");

    let summary = scan_path_with_config(root, &ScanConfig::default()).expect("scan");

    assert_eq!(summary.metrics.files_skipped_repopilotignore, 0);
    assert!(summary.repopilotignore_path.is_none());
}
