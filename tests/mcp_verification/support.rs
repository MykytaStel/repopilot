use serde_json::{Value, json};
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};

pub(super) fn client_with_form(root: &Path) -> InteractiveMcpClient {
    InteractiveMcpClient::start(
        root,
        "2025-11-25",
        json!({
            "elicitation": { "form": {} }
        }),
    )
}

pub(super) fn tool_call(id: u64, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": "repopilot_review_change", "arguments": arguments }
    })
}

pub(super) struct InteractiveMcpClient {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl InteractiveMcpClient {
    pub(super) fn start(root: &Path, protocol: &str, capabilities: Value) -> Self {
        Self::start_with_path(root, protocol, capabilities, None)
    }

    pub(super) fn start_with_path(
        root: &Path,
        protocol: &str,
        capabilities: Value,
        path: Option<OsString>,
    ) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_repopilot"));
        command
            .arg("mcp")
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(path) = path {
            command.env("PATH", path);
        }
        let mut child = command.spawn().expect("spawn MCP server");
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

    pub(super) fn send(&mut self, message: Value) {
        let stdin = self.stdin.as_mut().expect("MCP client stdin is open");
        writeln!(stdin, "{message}").expect("write MCP message");
        stdin.flush().expect("flush MCP message");
    }

    pub(super) fn receive(&mut self) -> Value {
        let mut line = String::new();
        let count = self.stdout.read_line(&mut line).expect("read MCP message");
        assert_ne!(count, 0, "MCP server closed stdout unexpectedly");
        serde_json::from_str(&line).expect("MCP message is JSON")
    }

    pub(super) fn receive_with_id(&mut self, id: u64) -> Value {
        loop {
            let message = self.receive();
            if message["id"] == id {
                return message;
            }
        }
    }

    pub(super) fn close(mut self) -> ExitStatus {
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
