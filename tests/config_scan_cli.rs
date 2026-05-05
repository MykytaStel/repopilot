use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn scan_uses_explicit_config_path_and_default_output_format() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");
    let config_path = temp.path().join("repopilot.toml");
    fs::write(
        &config_path,
        r#"
        [output]
        default_format = "json"
        "#,
    )
    .expect("failed to write config");

    let output = repopilot()
        .args(["scan", "."])
        .arg("--config")
        .arg(&config_path)
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("expected JSON output");
    assert_eq!(json["files_count"], 2);
}

#[test]
fn cli_threshold_overrides_config_threshold() {
    let temp = tempdir().expect("failed to create temp dir");
    let content = (0..11)
        .map(|index| format!("fn function_{index}() {{}}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(temp.path().join("medium.rs"), content).expect("failed to write file");
    let config_path = temp.path().join("repopilot.toml");
    fs::write(
        &config_path,
        r#"
        [architecture]
        max_file_lines = 10

        [output]
        default_format = "json"
        "#,
    )
    .expect("failed to write config");

    let output = repopilot()
        .args(["scan", ".", "--max-file-loc", "500"])
        .arg("--config")
        .arg(&config_path)
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("expected JSON output");
    let findings = json["findings"]
        .as_array()
        .expect("findings should be array");

    assert!(findings.iter().all(|finding| {
        finding["rule_id"]
            .as_str()
            .is_none_or(|rule_id| rule_id != "architecture.large-file")
    }));
}
