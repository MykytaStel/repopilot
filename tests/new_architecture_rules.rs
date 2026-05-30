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
            .artifacts
            .findings
            .iter()
            .any(|finding| finding.rule_id == "architecture.too-many-modules")
    );
}

#[test]
fn flat_tests_directory_is_not_reported_as_too_many_modules() {
    let temp = tempdir().expect("failed to create temp dir");
    let tests = temp.path().join("tests");
    fs::create_dir(&tests).expect("failed to create tests dir");

    for index in 0..3 {
        fs::write(
            tests.join(format!("case_{index}.rs")),
            "pub fn test_helper() {}\n",
        )
        .expect("failed to write test module");
    }

    let config = ScanConfig {
        max_directory_modules: 2,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan");

    assert!(
        summary
            .artifacts
            .findings
            .iter()
            .all(|finding| finding.rule_id != "architecture.too-many-modules")
    );
}

#[test]
fn docs_directory_is_not_reported_as_too_many_modules() {
    let temp = tempdir().expect("failed to create temp dir");
    let docs = temp.path().join("docs");
    fs::create_dir(&docs).expect("failed to create docs dir");

    for index in 0..3 {
        fs::write(docs.join(format!("guide_{index}.md")), "# Guide\n")
            .expect("failed to write doc");
    }

    let config = ScanConfig {
        max_directory_modules: 2,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan");

    assert!(
        summary
            .artifacts
            .findings
            .iter()
            .all(|finding| finding.rule_id != "architecture.too-many-modules")
    );
}

#[test]
fn documentation_files_do_not_inflate_module_count() {
    let temp = tempdir().expect("failed to create temp dir");
    let src = temp.path().join("src");
    fs::create_dir(&src).expect("failed to create src dir");

    for index in 0..2 {
        fs::write(
            src.join(format!("module_{index}.rs")),
            "pub fn value() {}\n",
        )
        .expect("failed to write module");
    }

    for index in 0..5 {
        fs::write(src.join(format!("note_{index}.md")), "# Note\n").expect("failed to write doc");
    }

    let config = ScanConfig {
        max_directory_modules: 2,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan");

    assert!(
        summary
            .artifacts
            .findings
            .iter()
            .all(|finding| finding.rule_id != "architecture.too-many-modules")
    );
}

#[test]
fn reports_deep_directory_nesting_only_above_threshold() {
    let temp = tempdir().expect("failed to create temp dir");
    let deep = temp.path().join("src");
    fs::create_dir_all(&deep).expect("failed to create nested dirs");
    fs::write(
        deep.join("feature.rs"),
        r#"
        fn foo() {
            if a {
                if b {
                    if c {
                        println!("nested");
                    }
                }
            }
        }
    "#,
    )
    .expect("failed to write file");

    let config = ScanConfig {
        max_directory_depth: 2,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan");
    assert!(
        summary
            .artifacts
            .findings
            .iter()
            .any(|finding| finding.rule_id == "architecture.deep-directory-nesting")
    );

    let config = ScanConfig {
        max_directory_depth: 10,
        ..ScanConfig::default()
    };
    let summary = scan_path_with_config(temp.path(), &config).expect("failed to scan");
    assert!(
        summary
            .artifacts
            .findings
            .iter()
            .all(|finding| finding.rule_id != "architecture.deep-directory-nesting")
    );
}
