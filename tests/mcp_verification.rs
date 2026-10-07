#![cfg(unix)]

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};

#[path = "mcp_verification/basic.rs"]
mod basic;
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

fn client_with_form(root: &Path) -> InteractiveMcpClient {
    InteractiveMcpClient::start(
        root,
        "2025-11-25",
        json!({
            "elicitation": { "form": {} }
        }),
    )
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

struct InteractiveMcpClient {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl InteractiveMcpClient {
    fn start(root: &Path, protocol: &str, capabilities: Value) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_repopilot"))
            .arg("mcp")
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn MCP server");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let mut client = Self {
            child,
            stdin: Some(stdin),
            stdout,
        };
        client.send(json!({
            "jsonrpc": "2.0",
            "id": "initialize",
            "method": "initialize",
            "params": { "protocolVersion": protocol, "capabilities": capabilities }
        }));
        assert_eq!(client.receive()["id"], "initialize");
        client.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        client
    }

    fn send(&mut self, message: Value) {
        let stdin = self.stdin.as_mut().expect("MCP client stdin is open");
        writeln!(stdin, "{message}").expect("write MCP message");
        stdin.flush().expect("flush MCP message");
    }

    fn receive(&mut self) -> Value {
        let mut line = String::new();
        let count = self.stdout.read_line(&mut line).expect("read MCP message");
        assert_ne!(count, 0, "MCP server closed stdout unexpectedly");
        serde_json::from_str(&line).expect("MCP message is JSON")
    }

    fn receive_with_id(&mut self, id: u64) -> Value {
        loop {
            let message = self.receive();
            if message["id"] == id {
                return message;
            }
        }
    }

    fn close(mut self) -> ExitStatus {
        self.stdin.take();
        self.child.wait().expect("wait for MCP server")
    }
}

impl Drop for InteractiveMcpClient {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
