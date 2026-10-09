//! CLI/MCP contract for the quiet-fallback review signal.

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use tempfile::tempdir;

fn cli_review(root: &Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["review", ".", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("run CLI review");
    assert!(
        output.status.success(),
        "CLI review failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("CLI review JSON")
}

fn start_mcp(root: &Path) -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .arg("mcp")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn MCP server");
    let stdin = child.stdin.take().expect("MCP stdin");
    let stdout = BufReader::new(child.stdout.take().expect("MCP stdout"));
    (child, stdin, stdout)
}

fn send(stdin: &mut ChildStdin, request: &Value) {
    writeln!(stdin, "{request}").expect("write MCP request");
    stdin.flush().expect("flush MCP request");
}

fn receive(stdout: &mut BufReader<ChildStdout>) -> Value {
    let mut line = String::new();
    stdout.read_line(&mut line).expect("read MCP response");
    assert!(!line.is_empty(), "MCP server closed before responding");
    serde_json::from_str(&line).expect("MCP response JSON")
}

fn setup_change(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
    fs::create_dir_all(root.join("src")).expect("create src directory");
    fs::write(
        root.join("src/thumbnail.js"),
        "export async function thumbnail(input) {\n  try { return await transform(input); } catch (error) { throw error; }\n}\n",
    )
    .expect("write original source");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "initial"]);
    fs::write(
        root.join("src/thumbnail.js"),
        r#"export async function thumbnail(input) {
  try {
    return await transform(input);
  } catch {
    // Keep the input when transformation fails.
  }
  return Buffer.from(input);
}
"#,
    )
    .expect("write changed source");
}

fn setup_python_change(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
    fs::create_dir_all(root.join("src")).expect("create src directory");
    fs::write(
        root.join("src/thumbnail.py"),
        "def thumbnail(image):\n    try:\n        return native_transform(image)\n    except ImportError:\n        raise\n",
    )
    .expect("write original source");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "initial"]);
    fs::write(
        root.join("src/thumbnail.py"),
        "def thumbnail(image):\n    try:\n        return native_transform(image)\n    except ImportError:\n        pass\n    return image\n",
    )
    .expect("write changed source");
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn signal(report: &Value) -> Value {
    report["tiered_signals"]["maybe"]
        .as_array()
        .expect("maybe signals")
        .iter()
        .find(|signal| signal["kind"] == "behavioral.quiet-fallback-introduced")
        .expect("quiet fallback signal")
        .clone()
}

#[test]
fn cli_and_mcp_publish_the_same_non_gating_signal() {
    let temp = tempdir().expect("temp directory");
    setup_change(temp.path());
    let cli_signal = signal(&cli_review(temp.path()));

    let (mut child, mut stdin, mut stdout) = start_mcp(temp.path());
    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": "initialize",
            "method": "initialize",
            "params": { "protocolVersion": "2025-11-25" }
        }),
    );
    let _ = receive(&mut stdout);
    send(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    );
    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": "review",
            "method": "tools/call",
            "params": {
                "name": "repopilot_review_change",
                "arguments": { "path": ".", "detail": "full" }
            }
        }),
    );
    let response = receive(&mut stdout);
    let mcp_signal = signal(&response["result"]["structuredContent"]);

    assert_eq!(mcp_signal, cli_signal);
    assert_eq!(cli_signal["line"], 4);
    assert_eq!(cli_signal["confidence"], "MEDIUM");
    assert_eq!(cli_signal["tier"], "maybe-sensitive");
    assert_eq!(cli_signal["gate_eligible"], false);
    assert!(cli_signal["detail"].as_str().unwrap().contains("transform"));
    assert!(
        cli_signal["detail"]
            .as_str()
            .unwrap()
            .contains("return line 7")
    );

    drop(stdin);
    assert!(child.wait().expect("wait for MCP server").success());
}

#[test]
fn python_cli_and_mcp_publish_the_same_non_gating_signal() {
    let temp = tempdir().expect("temp directory");
    setup_python_change(temp.path());
    let cli_signal = signal(&cli_review(temp.path()));

    let (mut child, mut stdin, mut stdout) = start_mcp(temp.path());
    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": "initialize",
            "method": "initialize",
            "params": { "protocolVersion": "2025-11-25" }
        }),
    );
    let _ = receive(&mut stdout);
    send(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    );
    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": "review",
            "method": "tools/call",
            "params": {
                "name": "repopilot_review_change",
                "arguments": { "path": ".", "detail": "full" }
            }
        }),
    );
    let response = receive(&mut stdout);
    let mcp_signal = signal(&response["result"]["structuredContent"]);

    assert_eq!(mcp_signal, cli_signal);
    assert_eq!(cli_signal["line"], 5);
    assert_eq!(cli_signal["confidence"], "MEDIUM");
    assert_eq!(cli_signal["tier"], "maybe-sensitive");
    assert_eq!(cli_signal["gate_eligible"], false);
    assert!(
        cli_signal["detail"]
            .as_str()
            .unwrap()
            .contains("native_transform")
    );

    drop(stdin);
    assert!(child.wait().expect("wait for MCP server").success());
}
