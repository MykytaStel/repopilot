use repopilot::scan::config::ScanConfig;
use repopilot::scan::workspace_scan::scan_workspace_with_config;
use std::fs;
use tempfile::tempdir;

#[test]
fn library_workspace_scan_merges_packages_without_duplicate_findings() {
    let temp = tempdir().expect("failed to create temp dir");
    let root = temp.path();
    let package_root = root.join("packages/app");
    fs::create_dir_all(package_root.join("src")).expect("failed to create package src");
    fs::write(
        root.join("package.json"),
        r#"{"workspaces":["packages/*"]}"#,
    )
    .expect("failed to write root package.json");
    fs::write(package_root.join("package.json"), r#"{"name":"app"}"#)
        .expect("failed to write package.json");
    fs::write(
        package_root.join("src/user.js"),
        "export const value = 1;\n",
    )
    .expect("failed to write source file");

    let summary = scan_workspace_with_config(root, &ScanConfig::default()).expect("workspace scan");
    let source_findings = summary
        .findings
        .iter()
        .filter(|finding| {
            finding.rule_id == "testing.source-without-test"
                && finding
                    .evidence
                    .first()
                    .is_some_and(|evidence| evidence.path.ends_with("packages/app/src/user.js"))
        })
        .collect::<Vec<_>>();

    assert_eq!(
        source_findings.len(),
        1,
        "workspace package source finding must only be emitted once"
    );
    assert_eq!(source_findings[0].workspace_package.as_deref(), Some("app"));
}
