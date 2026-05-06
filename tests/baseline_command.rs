use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn baseline_create_creates_default_file_and_parent_directory() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");

    let output = repopilot()
        .args(["baseline", "create", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run baseline create");

    assert!(output.status.success());
    assert!(temp.path().join(".repopilot").is_dir());
    assert!(temp.path().join(".repopilot/baseline.json").is_file());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RepoPilot Baseline"));
    assert!(stdout.contains("Scanned path: ."));
    assert!(stdout.contains("Baseline written to: .repopilot/baseline.json"));
    assert!(stdout.contains("- High: 1"));
}

#[test]
fn baseline_create_does_not_overwrite_existing_file_without_force() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");
    fs::create_dir_all(temp.path().join(".repopilot")).expect("failed to create baseline dir");
    let baseline_path = temp.path().join(".repopilot/baseline.json");
    fs::write(&baseline_path, "sentinel\n").expect("failed to write sentinel baseline");

    let output = repopilot()
        .args(["baseline", "create", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run baseline create");

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(&baseline_path).expect("failed to read baseline"),
        "sentinel\n"
    );
}

#[test]
fn baseline_create_overwrites_existing_file_with_force() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");
    fs::create_dir_all(temp.path().join(".repopilot")).expect("failed to create baseline dir");
    let baseline_path = temp.path().join(".repopilot/baseline.json");
    fs::write(&baseline_path, "sentinel\n").expect("failed to write sentinel baseline");

    let output = repopilot()
        .args(["baseline", "create", ".", "--force"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run baseline create");

    assert!(output.status.success());
    let baseline: Value =
        serde_json::from_str(&fs::read_to_string(&baseline_path).expect("failed to read baseline"))
            .expect("baseline should be JSON");
    assert_eq!(baseline["schema_version"], 1);
    assert_ne!(baseline, Value::String("sentinel\n".to_string()));
}

#[test]
fn baseline_create_supports_explicit_output_path() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");
    let baseline_path = temp.path().join("baseline.json");

    let output = repopilot()
        .args(["baseline", "create", ".", "--output"])
        .arg(&baseline_path)
        .current_dir(temp.path())
        .output()
        .expect("failed to run baseline create");

    assert!(output.status.success());
    assert!(baseline_path.is_file());
}

#[test]
fn baseline_create_stores_stable_keys_and_repo_relative_paths() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");

    let output = repopilot()
        .args(["baseline", "create", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run baseline create");

    assert!(output.status.success());

    let baseline: Value = serde_json::from_str(
        &fs::read_to_string(temp.path().join(".repopilot/baseline.json"))
            .expect("failed to read baseline"),
    )
    .expect("baseline should be JSON");
    let findings = baseline["findings"]
        .as_array()
        .expect("findings should be an array");
    let secret = findings
        .iter()
        .find(|finding| finding["rule_id"] == "security.secret-candidate")
        .expect("expected secret finding");

    assert_eq!(secret["key"], "security.secret-candidate:src/config.rs:1");
    assert_eq!(secret["path"], "src/config.rs");
    assert!(
        !secret["key"]
            .as_str()
            .unwrap()
            .contains(temp.path().to_str().unwrap())
    );
}

fn write_project_with_secret(root: &std::path::Path, module: &str) {
    fs::create_dir_all(root.join("src")).expect("failed to create src dir");
    fs::create_dir_all(root.join("tests")).expect("failed to create tests dir");
    fs::write(
        root.join(format!("src/{module}.rs")),
        "const API_KEY: &str = \"abc12345\";\n",
    )
    .expect("failed to write source file");
    fs::write(
        root.join(format!("tests/{module}.rs")),
        "fn covers_module() {}\n",
    )
    .expect("failed to write test file");
}
