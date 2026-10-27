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

fn setup_cross_file_handler_move(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
    fs::create_dir_all(root.join("src/transport")).expect("create source directories");
    fs::write(
        root.join("src/mcp.ts"),
        "export async function publish() {\n  try {\n    await send();\n  } catch (error) {\n    return fallback(error);\n  }\n}\n\nexport function status() { return true; }\n",
    )
    .expect("write original module");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "initial"]);

    fs::write(
        root.join("src/mcp.ts"),
        "export function status() { return true; }\n",
    )
    .expect("remove moved function from original module");
    fs::write(
        root.join("src/transport/publish.ts"),
        "export async function publish() {\n    try {\n        await send();\n    } catch (error) {\n        return fallback(error);\n    }\n}\n",
    )
    .expect("write moved function into transport module");
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

fn error_handling_signals(report: &Value) -> Vec<Value> {
    ["definitely", "maybe", "noise"]
        .into_iter()
        .filter_map(|tier| report["tiered_signals"][tier].as_array())
        .flatten()
        .filter(|signal| signal["kind"] == "behavioral.error-handling-removed")
        .cloned()
        .collect()
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

#[test]
fn cli_and_mcp_match_exact_handler_moves_but_keep_changed_handler_removals() {
    let temp = tempdir().expect("temp directory");
    setup_cross_file_handler_move(temp.path());
    let cli_signals = error_handling_signals(&cli_review(temp.path()));
    assert!(
        cli_signals.is_empty(),
        "the moved handler should remain accounted for: {cli_signals:#?}"
    );

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
    let mcp_signals = error_handling_signals(&response["result"]["structuredContent"]);

    assert_eq!(mcp_signals, cli_signals);

    fs::write(
        temp.path().join("src/transport/publish.ts"),
        "export async function publish() {\n    try {\n        await send();\n    } catch (error) {\n        throw error;\n    }\n}\n",
    )
    .expect("change the destination handler instead of moving it exactly");
    let cli_remaining = error_handling_signals(&cli_review(temp.path()));
    assert_eq!(
        cli_remaining.len(),
        1,
        "a changed destination handler must not cancel the removal: {cli_remaining:#?}"
    );

    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": "review-changed-handler",
            "method": "tools/call",
            "params": {
                "name": "repopilot_review_change",
                "arguments": { "path": ".", "detail": "full" }
            }
        }),
    );
    let changed_response = receive(&mut stdout);
    let mcp_remaining = error_handling_signals(&changed_response["result"]["structuredContent"]);
    assert_eq!(mcp_remaining, cli_remaining);

    fs::write(
        temp.path().join("src/transport/publish.ts"),
        "export async function publishForRetry() {\n    try {\n        await send();\n    } catch (error) {\n        return fallback(error);\n    }\n}\n",
    )
    .expect("move identical handler under a different function");
    let different_owner = error_handling_signals(&cli_review(temp.path()));
    assert!(
        different_owner.is_empty(),
        "an unchanged handler remains accounted for after extraction into a differently named function: {different_owner:#?}"
    );

    fs::write(
        temp.path().join("src/transport/publish.ts"),
        "export async function publish() {\n    try { await send(); } catch (error) { return fallback(error); }\n}\nexport async function publish() {\n    try { await send(); } catch (error) { return fallback(error); }\n}\n",
    )
    .expect("write ambiguous duplicate handlers");
    let ambiguous = error_handling_signals(&cli_review(temp.path()));
    assert_eq!(
        ambiguous.len(),
        1,
        "duplicate destinations must leave the removal visible: {ambiguous:#?}"
    );

    drop(stdin);
    assert!(child.wait().expect("wait for MCP server").success());
}
