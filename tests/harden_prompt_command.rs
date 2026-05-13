use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

fn write_sample_project(root: &std::path::Path) {
    let src = root.join("src");
    fs::create_dir_all(&src).expect("failed to create src dir");
    fs::write(
        src.join("config.rs"),
        r#"const API_KEY: &str = "sk_live_123456789abcdef";
"#,
    )
    .expect("failed to write config file");
    fs::write(src.join("todo.rs"), "// TODO: remove this marker\n")
        .expect("failed to write todo file");
}

fn write_medium_signal_project(root: &std::path::Path) {
    let src = root.join("src");
    fs::create_dir_all(&src).expect("failed to create src dir");
    let content = (0..20)
        .map(|index| format!("pub fn function_{index}() {{}}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(src.join("large_a.rs"), &content).expect("failed to write large_a");
    fs::write(src.join("large_b.rs"), &content).expect("failed to write large_b");
    fs::write(
        root.join("repopilot.toml"),
        r#"
        [architecture]
        max_file_lines = 10
        "#,
    )
    .expect("failed to write config");
}

#[test]
fn harden_default_output_succeeds() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    let output = repopilot()
        .args(["harden", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot harden");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("# RepoPilot Harden Plan"));
    assert!(stdout.contains("P0 - Immediate risk"));
    assert!(stdout.contains("Possible secret detected"));
    assert!(stdout.contains("Move the value to an environment variable or secrets manager"));
    assert!(stdout.contains("## Verify"));
}

#[test]
fn harden_focus_security_excludes_quality_findings() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    let output = repopilot()
        .args(["harden", ".", "--focus", "security", "--budget", "2k"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot harden");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Possible secret detected"));
    assert!(!stdout.contains("TODO marker"));
}

#[test]
fn harden_groups_repeated_medium_findings_by_rule() {
    let temp = tempdir().expect("failed to create temp dir");
    write_medium_signal_project(temp.path());

    let output = repopilot()
        .args(["harden", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot harden");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("File exceeds recommended size (2 findings)"));
    assert!(stdout.contains("Rule: `architecture.large-file`"));
    assert!(stdout.contains("`./src/large_a.rs:1`"));
    assert!(stdout.contains("`./src/large_b.rs:1`"));
}

#[test]
fn prompt_default_output_succeeds() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    let output = repopilot()
        .args(["prompt", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot prompt");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("# RepoPilot Remediation Prompt"));
    assert!(stdout.contains("You are an AI coding assistant"));
    assert!(stdout.contains("## Operating Rules"));
    assert!(stdout.contains("## Triage Order"));
    assert!(stdout.contains("## Verification Contract"));
    assert!(stdout.contains("## Final Response Format"));
    assert!(stdout.contains("# RepoPilot Vibe Check"));
    assert!(stdout.contains("Possible secret detected"));
    assert!(stdout.contains("Move the value to an environment variable or secrets manager"));
}

#[test]
fn harden_and_prompt_support_output_files() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());
    let harden_path = temp.path().join("harden.md");
    let prompt_path = temp.path().join("prompt.md");

    let harden_output = repopilot()
        .args(["harden", ".", "--output"])
        .arg(&harden_path)
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot harden");
    assert!(harden_output.status.success());
    assert!(harden_output.stdout.is_empty());

    let prompt_output = repopilot()
        .args(["prompt", ".", "--output"])
        .arg(&prompt_path)
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot prompt");
    assert!(prompt_output.status.success());
    assert!(prompt_output.stdout.is_empty());

    let harden = fs::read_to_string(harden_path).expect("failed to read harden output");
    let prompt = fs::read_to_string(prompt_path).expect("failed to read prompt output");
    assert!(harden.contains("# RepoPilot Harden Plan"));
    assert!(prompt.contains("# RepoPilot Remediation Prompt"));
}
