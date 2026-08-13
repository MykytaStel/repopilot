use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const B21_KIND: &str = "behavioral.removed-export-still-imported";

#[test]
fn same_line_alias_occurrences_each_replay_through_mcp() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export function loadUser() {}\n");
    write(
        root,
        "src/caller.ts",
        "import { loadUser as first, loadUser as second } from \"./api.ts\";\n",
    );
    commit_all(root, "before");
    write(root, "src/api.ts", "export function saveUserAccount() {}\n");

    let (mut child, mut stdin, mut stdout) = start_mcp(root);
    initialize_mcp(&mut stdin, &mut stdout);
    send(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":"review","method":"tools/call","params":{"name":"repopilot_review_change","arguments":{"path":".","detail":"full"}}}),
    );
    let review = receive(&mut stdout);
    let signals = review["result"]["structuredContent"]["tiered_signals"]["definitely"]
        .as_array()
        .expect("definitely signals")
        .iter()
        .filter(|signal| signal["kind"] == B21_KIND)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(signals.len(), 2, "{signals:#?}");
    let ids = signals
        .iter()
        .map(|signal| signal["signal_id"].as_str().expect("signal id"))
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 2);

    for signal in signals {
        let signal_id = signal["signal_id"].as_str().expect("signal id");
        send(
            &mut stdin,
            &json!({"jsonrpc":"2.0","id":signal_id,"method":"tools/call","params":{"name":"repopilot_explain_review_signal","arguments":{"signal_id":signal_id}}}),
        );
        let explanation = receive(&mut stdout);
        let replay = &explanation["result"]["structuredContent"];
        assert_eq!(replay["status"], "explained");
        assert_eq!(replay["signal"]["signal_id"], signal["signal_id"]);
        assert_eq!(replay["signal"]["detail"], signal["detail"]);
        assert_eq!(replay["signal"]["path"], "src/caller.ts");
        assert_eq!(replay["signal"]["target_path"], "src/api.ts");
    }

    drop(stdin);
    assert!(child.wait().expect("wait for MCP server").success());
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
