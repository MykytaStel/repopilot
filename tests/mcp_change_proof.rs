//! End-to-end contract for ChangeProof parity across stored MCP projections.

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

fn start_mcp(root: &Path) -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .arg("mcp")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn repopilot mcp");
    let stdin = child.stdin.take().expect("child stdin");
    let stdout = BufReader::new(child.stdout.take().expect("child stdout"));
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

fn initialize(stdin: &mut ChildStdin, stdout: &mut BufReader<ChildStdout>) {
    send(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": "initialize",
            "method": "initialize",
            "params": { "protocolVersion": "2025-11-25" }
        }),
    );
    let _ = receive(stdout);
    send(
        stdin,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    );
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .status()
        .expect("git available");
    assert!(status.success(), "git {args:?} failed");
}

fn setup_removed_export_change(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    fs::create_dir_all(root.join("src")).expect("src directory");
    fs::write(root.join("src/api.ts"), "export function loadUser() {}\n").expect("api");
    fs::write(
        root.join("src/caller.ts"),
        "import { loadUser } from \"./api.ts\";\nloadUser();\n",
    )
    .expect("caller");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "before"]);
    fs::write(
        root.join("src/api.ts"),
        "export function saveUserAccount() {}\n",
    )
    .expect("changed api");
}

fn removed_export_signal(report: &Value) -> &Value {
    report["tiered_signals"]["definitely"]
        .as_array()
        .expect("definitely signals")
        .iter()
        .find(|signal| signal["kind"] == "behavioral.removed-export-still-imported")
        .expect("removed export signal")
}

#[test]
fn stored_mcp_projections_share_the_canonical_change_proof() {
    let temp = tempfile::tempdir().expect("temp dir");
    setup_removed_export_change(temp.path());
    let (mut child, mut stdin, mut stdout) = start_mcp(temp.path());
    initialize(&mut stdin, &mut stdout);

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
    let review = receive(&mut stdout);
    let report = &review["result"]["structuredContent"];
    let proof = report["change_proof"].clone();
    assert!(proof.is_object(), "review publishes ChangeProof");
    let handle = review["result"]["analysisHandle"]
        .as_str()
        .expect("review analysis handle");

    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": "context",
            "method": "tools/call",
            "params": {
                "name": "repopilot_context",
                "arguments": { "path": ".", "analysis_handle": handle }
            }
        }),
    );
    let context = receive(&mut stdout);
    assert_eq!(context["result"]["isError"], false);
    assert_eq!(
        context["result"]["structuredContent"]["change_proof"],
        proof
    );
    assert!(
        context["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("Repository Facts Summary")),
        "context content remains Markdown"
    );

    let signal_id = removed_export_signal(report)["signal_id"]
        .as_str()
        .expect("signal id");
    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": "explain",
            "method": "tools/call",
            "params": {
                "name": "repopilot_explain_review_signal",
                "arguments": { "signal_id": signal_id, "analysis_handle": handle }
            }
        }),
    );
    let explanation = receive(&mut stdout);
    assert_eq!(
        explanation["result"]["structuredContent"]["change_proof"],
        proof
    );

    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": "analyses",
            "method": "resources/read",
            "params": { "uri": "repopilot://analyses" }
        }),
    );
    let analyses = receive(&mut stdout);
    let summary: Value = serde_json::from_str(
        analyses["result"]["contents"][0]["text"]
            .as_str()
            .expect("analysis summaries"),
    )
    .expect("analysis summary JSON");
    assert_eq!(summary[0]["change_proof"], proof);

    drop(stdin);
    assert!(child.wait().expect("wait for MCP server").success());
}
