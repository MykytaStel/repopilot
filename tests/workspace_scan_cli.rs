use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn workspace_scan_does_not_duplicate_package_file_findings() {
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
        package_root.join("src/index.js"),
        "export const value = 1;\n",
    )
    .expect("failed to write source file");

    let output = repopilot()
        .args(["scan", ".", "--workspace", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    let summary: Value = serde_json::from_slice(&output.stdout).expect("expected JSON output");
    assert_eq!(
        summary["files_count"].as_u64(),
        Some(3),
        "root scan must not count package files a second time"
    );

    let findings = summary["findings"]
        .as_array()
        .expect("findings must be an array");
    let source_findings: Vec<_> = findings
        .iter()
        .filter(|finding| {
            finding["rule_id"] == "testing.source-without-test"
                && finding["evidence"][0]["path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("packages/app/src/index.js"))
        })
        .collect();

    assert_eq!(
        source_findings.len(),
        1,
        "workspace package source finding must only be emitted once"
    );
    assert_eq!(
        source_findings[0]["workspace_package"].as_str(),
        Some("app")
    );
}
