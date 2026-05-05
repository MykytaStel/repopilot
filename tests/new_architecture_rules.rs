use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use std::fs;
use tempfile::tempdir;

#[test]
fn reports_directory_with_too_many_modules() {
    let temp = tempdir().expect("failed to create temp dir");
    let src = temp.path().join("src");
    fs::create_dir(&src).expect("failed to create src dir");

    for index in 0..3 {
        fs::write(
            src.join(format!("module_{index}.rs")),
            "pub fn value() {}\n",
        )
        .expect("failed to write module");
    }

    let config = ScanConfig {
        max_directory_modules: 2,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan");

    assert!(
        summary
            .findings
            .iter()
            .any(|finding| finding.rule_id == "architecture.too-many-modules")
    );
}

#[test]
fn reports_deep_nesting_only_above_threshold() {
    let temp = tempdir().expect("failed to create temp dir");
    let deep = temp.path().join("src/a/b/c");
    fs::create_dir_all(&deep).expect("failed to create nested dirs");
    fs::write(deep.join("feature.rs"), "pub fn value() {}\n").expect("failed to write file");

    let config = ScanConfig {
        max_directory_depth: 2,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan");
    assert!(
        summary
            .findings
            .iter()
            .any(|finding| finding.rule_id == "architecture.deep-nesting")
    );

    let config = ScanConfig {
        max_directory_depth: 10,
        ..ScanConfig::default()
    };
    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan");
    assert!(
        summary
            .findings
            .iter()
            .all(|finding| finding.rule_id != "architecture.deep-nesting")
    );
}
