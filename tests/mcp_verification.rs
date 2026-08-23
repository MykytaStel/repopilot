#![cfg(unix)]

use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn absent_empty_and_duplicate_selection_preserve_explicit_execution() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf x >> verification-runs"]
"#,
    );
    let responses = run_mcp(
        temp.path(),
        vec![
            tool_call(1, json!({ "path": ".", "detail": "full" })),
            tool_call(2, json!({ "path": ".", "detail": "full", "verify": [] })),
            tool_call(
                3,
                json!({
                    "path": ".",
                    "detail": "full",
                    "verify": ["unit", "unit"]
                }),
            ),
        ],
    );

    for id in [1, 2] {
        let result = result_for(&responses, id);
        assert_eq!(result["isError"], false);
        assert!(
            result["structuredContent"]["merge_readiness"]
                .get("verification")
                .is_none()
        );
    }
    let selected = result_for(&responses, 3);
    assert_eq!(
        selected["structuredContent"]["merge_readiness"]["verification"]
            .as_array()
            .expect("verification outcomes")
            .len(),
        1
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("verification-runs")).expect("marker"),
        "x"
    );
}

#[test]
fn unknown_selector_fails_before_spawn_and_publication() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "known"
role = "test"
program = "sh"
args = ["-c", "printf spawned > should-not-exist"]
"#,
    );
    let responses = run_mcp(
        temp.path(),
        vec![
            tool_call(4, json!({ "path": ".", "verify": ["unknown"] })),
            json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "resources/read",
                "params": { "uri": "repopilot://analyses" }
            }),
        ],
    );

    let result = result_for(&responses, 4);
    assert_eq!(result["isError"], true);
    assert!(
        result["content"][0]["text"]
            .as_str()
            .expect("error")
            .contains("unknown verification check id")
    );
    assert!(!temp.path().join("should-not-exist").exists());
    assert_eq!(result_for(&responses, 5)["contents"][0]["text"], "[]");
}

#[test]
fn failed_check_is_blocked_publishable_evidence() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf failure >&2; exit 7"]
"#,
    );
    let responses = run_mcp(
        temp.path(),
        vec![tool_call(
            6,
            json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
        )],
    );

    let result = result_for(&responses, 6);
    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["merge_readiness"]["verification"][0]["status"],
        "failed"
    );
    assert_eq!(
        result["structuredContent"]["merge_readiness"]["verdict"],
        "blocked"
    );
    assert!(result["analysisHandle"].is_string());
}

fn verification_repo(config: &str) -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("temp dir");
    git(temp.path(), &["init", "-q"]);
    git(temp.path(), &["config", "user.email", "test@example.com"]);
    git(temp.path(), &["config", "user.name", "Test"]);
    fs::write(
        temp.path().join("lib.rs"),
        "pub fn value() -> usize { 1 }\n",
    )
    .expect("source");
    fs::write(temp.path().join("repopilot.toml"), config).expect("config");
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-qm", "initial"]);
    fs::write(
        temp.path().join("lib.rs"),
        "pub fn value() -> usize { 2 }\n",
    )
    .expect("change");
    temp
}

fn tool_call(id: u64, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": "repopilot_review_change", "arguments": arguments }
    })
}

fn run_mcp(root: &Path, requests: Vec<Value>) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .arg("mcp")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn MCP server");
    let mut stdin = child.stdin.take().expect("stdin");
    writeln!(
        stdin,
        "{}",
        json!({
            "jsonrpc": "2.0",
            "id": "init",
            "method": "initialize",
            "params": { "protocolVersion": "2025-11-25" }
        })
    )
    .expect("initialize");
    writeln!(
        stdin,
        "{}",
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        })
    )
    .expect("initialized");
    for request in requests {
        writeln!(stdin, "{request}").expect("request");
    }
    drop(stdin);

    let output = child.wait_with_output().expect("MCP output");
    assert!(output.status.success(), "MCP server failed");
    String::from_utf8(output.stdout)
        .expect("UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSON response"))
        .collect()
}

fn result_for(responses: &[Value], id: u64) -> &Value {
    &responses
        .iter()
        .find(|response| response["id"] == id)
        .unwrap_or_else(|| panic!("missing response {id}"))["result"]
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git");
    assert!(output.status.success(), "git {args:?} failed");
}
