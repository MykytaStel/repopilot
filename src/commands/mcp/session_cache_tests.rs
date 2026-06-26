use super::{ServerState, handle_tools_call, scan};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::Command;

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git is available");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn tool_text(response: super::jsonrpc::Response) -> String {
    response
        .result
        .expect("response result")
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .and_then(|content| content.get("text"))
        .and_then(Value::as_str)
        .expect("tool text")
        .to_string()
}

#[test]
fn scan_tool_does_not_return_same_session_cache_after_an_edit() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "t@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(root.join("src/lib.rs"), "pub fn live() -> i32 { 1 }\n").expect("source");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "init"]);

    let mut state = ServerState {
        root: root.canonicalize().expect("canonical root"),
        initialized: true,
        negotiated: true,
        ..ServerState::default()
    };
    let params = json!({
        "name": scan::TOOL_NAME,
        "arguments": {
            "path": ".",
            "profile": "strict",
            "filters": { "rules": ["security.secret-candidate"] }
        }
    });

    let first = tool_text(handle_tools_call(json!(1), &params, &mut state));
    assert!(
        !first.contains("security.secret-candidate"),
        "fixture should start without a secret finding"
    );

    fs::write(
        root.join("src/config.ts"),
        "export const API_KEY = \"abc123xyz987\";\n",
    )
    .expect("secret fixture");
    let second = tool_text(handle_tools_call(json!(2), &params, &mut state));

    assert!(
        second.contains("security.secret-candidate"),
        "the second scan must observe edits made in the same MCP session"
    );
}
