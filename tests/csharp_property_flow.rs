use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

#[test]
fn csharp_dirty_property_flow_retains_cli_mcp_provenance() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    init_repo(root);
    let before = "class Controller { void Find(SqlCommand cmd) {\nvar id = Request.Query[\"id\"];\ncmd.CommandText = \"SELECT 1\";\ncmd.ExecuteReader();\n} }\n";
    write(root, "src/Controller.cs", before);
    commit_all(root, "before");
    write(
        root,
        "src/Controller.cs",
        &before.replace("\"SELECT 1\"", "id"),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["review", ".", "--format", "json"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let cli: Value = serde_json::from_slice(&output.stdout).unwrap();
    let signal = taint(&cli);
    assert_eq!(signal["line"], 4);
    assert!(
        signal["detail"]
            .as_str()
            .unwrap()
            .contains("CommandText at line 3")
    );
    assert!(signal["detail"].as_str().unwrap().contains("at line 2"));
    let (mut child, mut stdin, mut stdout) = start_mcp(root);
    initialize_mcp(&mut stdin, &mut stdout);
    send(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":"review","method":"tools/call","params":{"name":"repopilot_review_change","arguments":{"path":".","detail":"full"}}}),
    );
    let response = receive(&mut stdout);
    let retained = taint(&response["result"]["structuredContent"]);
    assert_eq!(retained["detail"], signal["detail"]);
    assert_eq!(retained["signal_id"], signal["signal_id"]);
    send(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":"explain","method":"tools/call","params":{"name":"repopilot_explain_review_signal","arguments":{"signal_id":retained["signal_id"]}}}),
    );
    let explanation = receive(&mut stdout);
    assert_eq!(
        explanation["result"]["structuredContent"]["status"],
        "explained"
    );
    assert_eq!(
        explanation["result"]["structuredContent"]["signal"]["detail"],
        signal["detail"]
    );
    drop(stdin);
    assert!(child.wait().unwrap().success());
}

fn taint(value: &Value) -> &Value {
    ["definitely", "maybe", "noise"]
        .into_iter()
        .flat_map(|tier| value["tiered_signals"][tier].as_array().unwrap())
        .find(|signal| signal["family"] == "taint")
        .expect("property taint signal")
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
    let stdin = child.stdin.take().expect("child stdin");
    let stdout = BufReader::new(child.stdout.take().expect("child stdout"));
    (child, stdin, stdout)
}

fn initialize_mcp(stdin: &mut ChildStdin, stdout: &mut BufReader<ChildStdout>) {
    send(
        stdin,
        &json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-11-25"}}),
    );
    let _ = receive(stdout);
    send(
        stdin,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    );
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

fn init_repo(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent directory")).expect("create parent");
    fs::write(path, content).expect("write source");
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
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
