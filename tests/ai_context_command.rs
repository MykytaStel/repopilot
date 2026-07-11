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
    assert!(stdout.contains("## Repository Facts Summary"));
    assert!(stdout.contains("- Files: 2"));
    assert!(stdout.contains("- Files with detected language: 2"));
    assert!(stdout.contains("- Non-empty lines: 2"));
    assert!(stdout.contains("- Fact diagnostics: 0"));
    assert!(stdout.contains("- Rust: 2 files, 2 non-empty lines"));
    assert!(stdout.contains("Security"));
    assert!(stdout.contains("Possible secret detected"));
    assert!(stdout.contains("sk_...***"));
    assert!(!stdout.contains(&temp.path().display().to_string()));
}

#[test]
fn ai_context_handoff_includes_plan_rules_and_verify() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    let run_md = |args: &[&str]| {
        let output = repopilot()
            .args(args)
            .current_dir(temp.path())
            .output()
            .expect("failed to run repopilot ai context");
        assert!(output.status.success());
        String::from_utf8(output.stdout).expect("stdout should be UTF-8")
    };

    // Default handoff carries the task guidance and the prioritized plan.
    let full = run_md(&["ai", "context", "."]);
    assert!(full.contains("## How To Work"), "rules missing\n{full}");
    assert!(full.contains("## Remediation Plan"), "plan missing\n{full}");
    assert!(full.contains("## Verify"), "verification missing\n{full}");

    // --no-task drops the agent guidance but keeps the fact-based plan.
    let lean = run_md(&["ai", "context", ".", "--no-task"]);
    assert!(
        !lean.contains("## How To Work"),
        "rules should be hidden under --no-task\n{lean}"
    );
    assert!(
        !lean.contains("## Verify"),
        "verification should be hidden under --no-task\n{lean}"
    );
    assert!(
        lean.contains("## Remediation Plan"),
        "plan should remain under --no-task\n{lean}"
    );
    assert!(
        lean.contains("## Repository Facts Summary"),
        "facts summary should remain under --no-task\n{lean}"
    );
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
fn ai_context_json_format_is_structured_facts_aware_and_clean() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    let output = repopilot()
        .args(["ai", "context", ".", "--format", "json", "--budget", "2k"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai context");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    // stdout is pure JSON — no Markdown leaks in.
    assert!(
        !stdout.contains("# RepoPilot AI Context"),
        "markdown leaked:\n{stdout}"
    );

    let doc: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert_eq!(doc["schema_version"], 3);
    assert_eq!(doc["artifact"]["kind"], "repopilot-analysis");
    assert_eq!(doc["artifact"]["version"], 1);
    assert_eq!(doc["artifact"]["source"], "ai-context");
    // Phase: facts come through the real CLI path (the golden passes None).
    assert_eq!(
        doc["facts"]["total_files"], 2,
        "facts present from the CLI: {doc}"
    );

    // A finding carries the structured evidence an agent needs, not just a title.
    let findings = doc["findings"].as_array().expect("findings array");
    let secret = findings
        .iter()
        .find(|finding| finding["rule_id"] == "security.secret-candidate")
        .expect("secret finding present");
    assert!(secret["risk"]["score"].is_number(), "risk score present");
    assert!(
        secret["evidence"][0]["path"].is_string(),
        "evidence locations present: {secret}"
    );
    assert!(
        secret["decision"]["recommendation"].is_string(),
        "canonical decision recommendation present"
    );
    assert!(
        secret["decision"]["verification_plan"]["steps"]
            .as_array()
            .is_some_and(|steps| !steps.is_empty()),
        "verification plan present"
    );

    // Budget is real: either the estimate fits, or the doc is explicitly truncated.
    let approx = doc["budget"]["approx_tokens"]
        .as_u64()
        .expect("approx_tokens");
    let truncated = doc["budget"]["truncated"]
        .as_bool()
        .expect("truncated flag");
    assert!(
        approx <= 2048 || truncated,
        "budget ignored: approx={approx} truncated={truncated}"
    );
}

#[test]
fn ai_context_json_focus_filters_and_writes_output_file() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    // --focus security: every emitted finding is in the security category.
    let focused = repopilot()
        .args([
            "ai", "context", ".", "--focus", "security", "--format", "json",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai context");
    assert!(focused.status.success());
    let doc: serde_json::Value =
        serde_json::from_str(&String::from_utf8(focused.stdout).expect("utf-8")).expect("json");
    assert_eq!(doc["focus"], "security");
    assert!(
        doc["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .all(|finding| finding["category"] == "SECURITY"),
        "focus did not filter: {doc}"
    );

    // --output writes a valid JSON file and leaves stdout empty.
    let path = temp.path().join("ai-context.json");
    let written = repopilot()
        .args(["ai", "context", ".", "--format", "json", "--output"])
        .arg(&path)
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai context");
    assert!(written.status.success());
    assert!(
        written.stdout.is_empty(),
        "stdout should be empty when writing a file"
    );
    let contents = fs::read_to_string(&path).expect("failed to read JSON output file");
    let _: serde_json::Value = serde_json::from_str(&contents).expect("output file is valid JSON");
}

#[test]
fn ai_context_json_does_not_emit_breakdown_to_stderr() {
    let temp = tempdir().expect("failed to create temp dir");
    write_sample_project(temp.path());

    let output = repopilot()
        .args(["ai", "context", ".", "--format", "json", "--show-breakdown"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot ai context");

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !stderr.contains("tokens"),
        "JSON output must not print the Markdown token breakdown to stderr: {stderr}"
    );
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
