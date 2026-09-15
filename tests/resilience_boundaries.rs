use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn malformed_config_fails_closed_without_echoing_sensitive_content() {
    let temp = tempdir().expect("tempdir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("source");
    let secret = "synthetic-config-secret-value";
    fs::write(
        temp.path().join("bad.toml"),
        format!("[scan]\nmax_file_bytes = \"{secret}\"\n"),
    )
    .expect("malformed config");

    let output = repopilot()
        .args([
            "scan",
            ".",
            "--config",
            "bad.toml",
            "--format",
            "json",
            "--no-progress",
        ])
        .current_dir(temp.path())
        .output()
        .expect("run scan");

    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stdout.is_empty(),
        "failed config must not emit a report"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid config"), "stderr: {stderr}");
    assert!(
        !stderr.contains(secret),
        "malformed config must not echo sensitive values: {stderr}"
    );
}

#[test]
fn malformed_python_source_is_reported_without_a_supported_proof_claim() {
    let temp = tempdir().expect("tempdir");
    init_repo(temp.path());
    fs::write(temp.path().join("app.py"), "def ok():\n    return 1\n").expect("valid source");
    commit_all(temp.path(), "base");
    fs::write(temp.path().join("app.py"), "def broken(:\n    return 2\n")
        .expect("malformed source");

    let output = repopilot()
        .args(["review", ".", "--format", "json", "--no-progress"])
        .current_dir(temp.path())
        .output()
        .expect("run review");
    assert!(
        output.status.success(),
        "syntax warning should still produce a report: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_slice(&output.stdout).expect("review JSON");
    assert!(report["diagnostics"].as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item["code"] == "python.syntax-error")
    }));
    assert_eq!(report["change_proof"]["verdict"], "REVIEW");
    assert_eq!(report["evidence"]["class"], "suspicion");
    assert_ne!(report["evidence"]["class"], "supported-proof");
}

#[cfg(unix)]
#[test]
fn mcp_rejects_a_missing_path_beneath_a_dangling_symlink() {
    use std::os::unix::fs::symlink;

    let temp = tempdir().expect("tempdir");
    let target = temp.path().join("outside-that-does-not-exist");
    symlink(&target, temp.path().join("linked")).expect("dangling symlink");

    let responses = run_mcp(
        temp.path(),
        &[
            json!({
                "jsonrpc": "2.0",
                "id": "init",
                "method": "initialize",
                "params": { "protocolVersion": "2025-11-25" }
            }),
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            json!({
                "jsonrpc": "2.0",
                "id": "scan",
                "method": "tools/call",
                "params": {
                    "name": "repopilot_scan",
                    "arguments": { "path": "linked/new.json" }
                }
            }),
        ],
    );

    let result = &responses[1]["result"];
    assert_eq!(result["isError"], true);
    let message = result["content"][0]["text"].as_str().expect("error text");
    assert!(
        message.contains("unavailable") || message.contains("must stay within MCP root"),
        "unexpected path error: {message}"
    );
}

#[test]
fn mcp_malformed_config_does_not_echo_sensitive_content() {
    let temp = tempdir().expect("tempdir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("source");
    let secret = "synthetic-mcp-config-secret";
    fs::write(
        temp.path().join("bad.toml"),
        format!("[scan]\nmax_file_bytes = \"{secret}\"\n"),
    )
    .expect("malformed config");

    let responses = run_mcp(
        temp.path(),
        &[
            json!({
                "jsonrpc": "2.0",
                "id": "init",
                "method": "initialize",
                "params": { "protocolVersion": "2025-11-25" }
            }),
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            json!({
                "jsonrpc": "2.0",
                "id": "scan",
                "method": "tools/call",
                "params": {
                    "name": "repopilot_scan",
                    "arguments": { "path": ".", "config": "bad.toml" }
                }
            }),
        ],
    );

    let result = &responses[1]["result"];
    assert_eq!(result["isError"], true);
    let message = result["content"][0]["text"].as_str().expect("error text");
    assert!(
        message.contains("invalid config"),
        "unexpected config error: {message}"
    );
    assert!(
        !message.contains(secret),
        "MCP must not echo configuration values: {message}"
    );
}

fn run_mcp(root: &Path, requests: &[Value]) -> Vec<Value> {
    let mut child = repopilot()
        .arg("mcp")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn MCP");
    {
        let mut stdin = child.stdin.take().expect("MCP stdin");
        for request in requests {
            writeln!(stdin, "{request}").expect("write MCP request");
        }
    }
    let output = child.wait_with_output().expect("wait for MCP");
    assert!(output.status.success(), "MCP failed: {:?}", output.status);
    BufReader::new(output.stdout.as_slice())
        .lines()
        .map(|line| serde_json::from_str(&line.expect("MCP response line")))
        .collect::<Result<Vec<Value>, _>>()
        .expect("MCP response JSON")
}

fn init_repo(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "repopilot@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", message]);
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
