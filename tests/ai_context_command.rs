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
fn ai_context_default_output_succeeds() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    let output = repopilot()
        .args(["ai", "context", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai context");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("# RepoPilot AI Context"));
    assert!(stdout.contains("Security"));
    assert!(stdout.contains("Possible secret detected"));
    assert!(stdout.contains("sk_...***"));
    assert!(!stdout.contains(&temp.path().display().to_string()));
}

#[test]
fn ai_context_focus_security_with_budget_succeeds() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    let output = repopilot()
        .args([
            "ai", "context", ".", "--focus", "security", "--budget", "2k",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai context");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("Security"));
    assert!(stdout.contains("Possible secret detected"));
    assert!(!stdout.contains("TODO marker"));
    assert!(!stdout.contains("## Hot Files"));
}

#[test]
fn ai_context_no_header_succeeds() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    let output = repopilot()
        .args(["ai", "context", ".", "--no-header"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai context");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(!stdout.contains("# RepoPilot AI Context"));
    assert!(stdout.contains("Security"));
}

#[test]
fn ai_context_output_file_succeeds() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());
    let output_path = temp.path().join("ai-context.md");

    let output = repopilot()
        .args(["ai", "context", ".", "--output"])
        .arg(&output_path)
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai context");

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let rendered = fs::read_to_string(output_path).expect("failed to read AI context output");
    assert!(rendered.contains("# RepoPilot AI Context"));
    assert!(rendered.contains("Possible secret detected"));
}

#[test]
fn ai_context_uses_default_product_visibility() {
    let temp = tempdir().expect("failed to create temp dir");
    write_medium_signal_project(temp.path());

    let output = repopilot()
        .args(["ai", "context", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai context");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(!stdout.contains("## Top Recommendations"));
    assert!(!stdout.contains("Large file detected"));
}

#[test]
fn ai_context_rejects_unknown_focus() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");

    let output = repopilot()
        .args(["ai", "context", ".", "--focus", "securty"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai context");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("invalid value"));
    assert!(stderr.contains("security"));
}

#[test]
fn ai_context_rejects_unknown_budget() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");

    let output = repopilot()
        .args(["ai", "context", ".", "--budget", "banana"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai context");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("positive token count"));
}

#[test]
fn ai_context_rejects_zero_budget() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");

    let output = repopilot()
        .args(["ai", "context", ".", "--budget", "0"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai context");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("greater than zero"));
}
