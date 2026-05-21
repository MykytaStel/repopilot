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
fn ai_plan_default_output_succeeds() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    let output = repopilot()
        .args(["ai", "plan", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai plan");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("# RepoPilot AI Plan"));
    assert!(stdout.contains("P0 - Immediate risk"));
    assert!(stdout.contains("Possible secret detected"));
    assert!(stdout.contains("Move the value to an environment variable or secrets manager"));
    assert!(stdout.contains("## Context Risk Graph"));
    assert!(stdout.contains("### Edit Order"));
    assert!(stdout.contains("## Verify"));
}

#[test]
fn ai_plan_focus_security_excludes_quality_findings() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    let output = repopilot()
        .args(["ai", "plan", ".", "--focus", "security", "--budget", "2k"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai plan");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Possible secret detected"));
    assert!(!stdout.contains("TODO marker"));
}

#[test]
fn ai_plan_uses_default_product_visibility() {
    let temp = tempdir().expect("failed to create temp dir");
    write_medium_signal_project(temp.path());

    let output = repopilot()
        .args(["ai", "plan", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai plan");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("No findings matched the selected scope."));
    assert!(!stdout.contains("File exceeds recommended size in src"));
    assert!(!stdout.contains("Rule: `architecture.large-file`"));
}

#[test]
fn prompt_default_output_succeeds() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    let output = repopilot()
        .args(["ai", "prompt", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai prompt");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("# RepoPilot Remediation Prompt"));
    assert!(stdout.contains("You are an AI coding assistant"));
    assert!(stdout.contains("## Operating Rules"));
    assert!(stdout.contains("## Triage Order"));
    assert!(stdout.contains("## Verification Contract"));
    assert!(stdout.contains("## Final Response Format"));
    assert!(stdout.contains("# RepoPilot AI Context"));
    assert!(stdout.contains("Possible secret detected"));
    assert!(stdout.contains("Move the value to an environment variable or secrets manager"));
}

#[test]
fn ai_plan_and_prompt_support_output_files() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());
    let ai_plan_path = temp.path().join("ai-plan.md");
    let prompt_path = temp.path().join("prompt.md");

    let ai_plan_output = repopilot()
        .args(["ai", "plan", ".", "--output"])
        .arg(&ai_plan_path)
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai plan");
    assert!(ai_plan_output.status.success());
    assert!(ai_plan_output.stdout.is_empty());

    let prompt_output = repopilot()
        .args(["ai", "prompt", ".", "--output"])
        .arg(&prompt_path)
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai prompt");
    assert!(prompt_output.status.success());
    assert!(prompt_output.stdout.is_empty());

    let ai_plan = fs::read_to_string(ai_plan_path).expect("failed to read AI plan output");
    let prompt = fs::read_to_string(prompt_path).expect("failed to read prompt output");
    assert!(ai_plan.contains("# RepoPilot AI Plan"));
    assert!(prompt.contains("# RepoPilot Remediation Prompt"));
}
