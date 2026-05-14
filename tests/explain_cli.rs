use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn explain_outputs_context_json_for_functional_rust_code() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    let src = root.join("src").join("domain");
    fs::create_dir_all(&src).expect("create src dir");
    fs::write(
        src.join("users.rs"),
        "pub fn active_names(users: Vec<User>) -> Vec<String> {\n\
            users.into_iter().filter(|user| user.active).map(|user| user.name).collect()\n\
        }\n",
    )
    .expect("write rust file");

    let output = repopilot()
        .args([
            "inspect",
            "explain",
            "src/domain/users.rs",
            "--format",
            "json",
        ])
        .current_dir(root)
        .output()
        .expect("run explain");

    assert!(
        output.status.success(),
        "explain should pass, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");

    assert_eq!(json["context"]["language"], "rust");
    assert_eq!(json["context"]["language_support"], "rule-aware");

    let paradigms = json["context"]["paradigms"]
        .as_array()
        .expect("paradigms array");
    assert!(paradigms.iter().any(|value| value == "functional"));

    let roles = json["context"]["roles"].as_array().expect("roles array");
    assert!(roles.iter().any(|value| value == "domain"));
}

#[test]
fn explain_can_show_suppressed_rule_decision_for_rust_tests() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    let tests = root.join("tests");
    fs::create_dir_all(&tests).expect("create tests dir");
    fs::write(
        tests.join("parser_test.rs"),
        "#[test]\nfn parses() { let value = Some(1).unwrap(); assert_eq!(value, 1); }\n",
    )
    .expect("write rust test");

    let output = repopilot()
        .args([
            "inspect",
            "explain",
            "tests/parser_test.rs",
            "--rule",
            "language.rust.panic-risk",
            "--signal",
            "rust.unwrap",
            "--severity",
            "medium",
            "--format",
            "json",
        ])
        .current_dir(root)
        .output()
        .expect("run explain");

    assert!(
        output.status.success(),
        "explain should pass, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");

    assert_eq!(json["decision"]["rule_id"], "language.rust.panic-risk");
    assert_eq!(json["decision"]["signal"], "rust.unwrap");
    assert_eq!(json["decision"]["action"], "suppress");
}
