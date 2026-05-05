use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn init_creates_default_config() {
    let temp = tempdir().expect("failed to create temp dir");

    let output = repopilot()
        .arg("init")
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot init");

    assert!(output.status.success());

    let config = fs::read_to_string(temp.path().join("repopilot.toml"))
        .expect("failed to read generated config");

    assert!(config.contains("[scan]"));
    assert!(config.contains("[architecture]"));
    assert!(config.contains("[output]"));
}

#[test]
fn init_does_not_overwrite_existing_config_without_force() {
    let temp = tempdir().expect("failed to create temp dir");
    let config_path = temp.path().join("repopilot.toml");
    fs::write(&config_path, "sentinel = true\n").expect("failed to write existing config");

    let output = repopilot()
        .arg("init")
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot init");

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(config_path).expect("failed to read config"),
        "sentinel = true\n"
    );
}

#[test]
fn init_overwrites_existing_config_with_force() {
    let temp = tempdir().expect("failed to create temp dir");
    let config_path = temp.path().join("repopilot.toml");
    fs::write(&config_path, "sentinel = true\n").expect("failed to write existing config");

    let output = repopilot()
        .args(["init", "--force"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot init");

    assert!(output.status.success());

    let config = fs::read_to_string(config_path).expect("failed to read config");
    assert!(config.contains("# RepoPilot configuration file"));
    assert!(!config.contains("sentinel = true"));
}
