#![cfg(unix)]

use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::Command;

#[path = "mcp_verification/support.rs"]
mod support;
use support::*;
#[path = "mcp_verification/consent.rs"]
mod consent;
#[path = "mcp_verification/freshness.rs"]
mod freshness;

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

fn accept_prompt(client: &mut InteractiveMcpClient, call_id: u64) -> Value {
    let prompt = client.receive();
    assert_eq!(prompt["method"], "elicitation/create");
    client.send(json!({
        "jsonrpc": "2.0",
        "id": prompt["id"],
        "result": { "action": "accept", "content": { "approve": true } }
    }));
    client.receive_with_id(call_id)["result"].clone()
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
    let mut client = client_with_form(temp.path());
    client.send(tool_call(
        6,
        json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
    ));
    let result = accept_prompt(&mut client, 6);
    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["merge_readiness"]["verification"]
            .as_array()
            .expect("verification outcomes")
            .len(),
        1
    );
    assert_eq!(
        result["structuredContent"]["merge_readiness"]["verification"][0]["status"],
        "failed"
    );
    assert_eq!(
        result["structuredContent"]["merge_readiness"]["verification"][0]["exit_code"],
        7
    );
    assert_eq!(
        result["structuredContent"]["merge_readiness"]["verification"][0]["stderr_excerpt"],
        "failure"
    );
    assert_eq!(
        result["structuredContent"]["merge_readiness"]["verification"][0]["stdout_excerpt"],
        ""
    );
    assert_eq!(
        result["structuredContent"]["merge_readiness"]["verification"][0]["stderr_truncated"],
        false
    );
    assert_eq!(
        result["structuredContent"]["merge_readiness"]["verdict"],
        "blocked"
    );
    assert!(result["analysisHandle"].is_string());
    assert!(client.close().success());
}

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
    let mut client = client_with_form(temp.path());
    client.send(tool_call(1, json!({ "path": ".", "detail": "full" })));
    let absent = client.receive_with_id(1)["result"].clone();
    client.send(tool_call(
        2,
        json!({ "path": ".", "detail": "full", "verify": [] }),
    ));
    let empty = client.receive_with_id(2)["result"].clone();

    for result in [absent, empty] {
        assert_eq!(result["isError"], false);
        assert!(
            result["structuredContent"]["merge_readiness"]
                .get("verification")
                .is_none()
        );
        assert!(result["analysisHandle"].is_string());
    }
    assert!(!temp.path().join("verification-runs").exists());

    client.send(tool_call(
        3,
        json!({
            "path": ".",
            "detail": "full",
            "verify": ["unit", "unit"]
        }),
    ));
    let selected = accept_prompt(&mut client, 3);
    assert_eq!(
        selected["structuredContent"]["merge_readiness"]["verification"]
            .as_array()
            .expect("verification outcomes")
            .len(),
        1
    );
    assert_eq!(
        selected["structuredContent"]["merge_readiness"]["verification"][0]["status"],
        "passed"
    );
    assert_eq!(
        selected["structuredContent"]["merge_readiness"]["verification"][0]["check_id"],
        "unit"
    );
    assert_eq!(
        selected["structuredContent"]["merge_readiness"]["verification"][0]["role"],
        "test"
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("verification-runs")).expect("marker"),
        "x"
    );
    assert!(client.close().success());
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
    let mut client = InteractiveMcpClient::start(temp.path(), "2025-11-25", json!({}));
    client.send(tool_call(4, json!({ "path": ".", "verify": ["unknown"] })));
    let result = client.receive_with_id(4)["result"].clone();
    assert_eq!(result["isError"], true);
    assert!(
        result["content"][0]["text"]
            .as_str()
            .expect("error")
            .contains("unknown verification check id")
    );
    assert!(!temp.path().join("should-not-exist").exists());
    client.send(json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "resources/read",
        "params": { "uri": "repopilot://analyses" }
    }));
    assert_eq!(
        client.receive_with_id(5)["result"]["contents"][0]["text"],
        "[]"
    );
    assert!(client.close().success());
}

#[test]
fn opted_in_cache_is_shared_by_mcp_calls_and_remains_publishable() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "mkdir -p .repopilot/cache; printf x >> .repopilot/cache/mcp-verification-runs"]
cache = { enabled = true }
"#,
    );
    let mut client = client_with_form(temp.path());
    client.send(tool_call(
        7,
        json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
    ));
    let first = accept_prompt(&mut client, 7);
    client.send(tool_call(
        8,
        json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
    ));
    let second = accept_prompt(&mut client, 8);
    assert!(
        first["structuredContent"]["merge_readiness"]["verification"][0]
            .get("reused")
            .is_none()
    );
    assert_eq!(
        second["structuredContent"]["merge_readiness"]["verification"][0]["reused"],
        true
    );
    assert!(first["analysisHandle"].is_string());
    assert!(second["analysisHandle"].is_string());
    assert_eq!(
        fs::read_to_string(temp.path().join(".repopilot/cache/mcp-verification-runs"))
            .expect("marker"),
        "x"
    );
    assert!(client.close().success());
}
