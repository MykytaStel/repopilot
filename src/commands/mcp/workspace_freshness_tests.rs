use super::{ServerState, context, explain_file, handle_tools_call, review_change, scan};
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

fn state(root: &Path) -> ServerState {
    ServerState {
        root: root.canonicalize().expect("canonical root"),
        initialized: true,
        negotiated: true,
        ..ServerState::default()
    }
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
fn scan_tool_observes_an_edit_in_the_same_session() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "t@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(root.join("src/lib.rs"), "pub fn live() -> i32 { 1 }\n").expect("source");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "init"]);

    let mut state = state(root);
    let params = json!({
        "name": scan::TOOL_NAME,
        "arguments": {
            "path": ".",
            "profile": "strict",
            "filters": { "rules": ["security.secret-candidate"] }
        }
    });

    let first = tool_text(handle_tools_call(json!(1), &params, &mut state));
    assert!(!first.contains("security.secret-candidate"));

    fs::write(
        root.join("src/config.ts"),
        "export const API_KEY = \"abc123xyz987\";\n",
    )
    .expect("secret fixture");

    let second = tool_text(handle_tools_call(json!(2), &params, &mut state));
    assert!(second.contains("security.secret-candidate"));
    assert_eq!(state.last_scan.as_deref(), Some(second.as_str()));
}

#[test]
fn review_change_observes_an_edit_and_updates_last_review() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "t@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(root.join("src/lib.rs"), "pub fn live() -> i32 { 1 }\n").expect("source");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "init"]);

    fs::write(root.join("src/lib.rs"), "pub fn live() -> i32 { 2 }\n").expect("safe edit");

    let mut state = state(root);
    let params = json!({
        "name": review_change::TOOL_NAME,
        "arguments": {
            "path": ".",
            "scope": "changed",
            "profile": "strict",
            "detail": "full",
            "filters": { "rules": ["language.rust.panic-risk"] }
        }
    });

    let first = tool_text(handle_tools_call(json!(3), &params, &mut state));
    assert!(!first.contains("language.rust.panic-risk"));

    fs::write(
        root.join("src/lib.rs"),
        "pub fn live() -> i32 { panic!(\"boom\") }\n",
    )
    .expect("panic edit");

    let second = tool_text(handle_tools_call(json!(4), &params, &mut state));
    assert!(second.contains("language.rust.panic-risk"));
    assert_eq!(state.last_review.as_deref(), Some(second.as_str()));
}

#[test]
fn context_observes_findings_added_after_the_first_call() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(root.join("src/lib.rs"), "pub fn live() -> i32 { 1 }\n").expect("source");

    let mut state = state(root);
    let params = json!({
        "name": context::TOOL_NAME,
        "arguments": {
            "path": ".",
            "focus": "security",
            "profile": "strict",
            "budget": 8192
        }
    });

    let first = tool_text(handle_tools_call(json!(5), &params, &mut state));
    assert!(!first.contains("security.secret-candidate"));

    fs::write(
        root.join("src/config.ts"),
        "export const API_KEY = \"abc123xyz987\";\n",
    )
    .expect("secret fixture");

    let second = tool_text(handle_tools_call(json!(6), &params, &mut state));
    assert!(second.contains("security.secret-candidate"));
}

#[test]
fn explain_file_observes_manifest_changes_after_the_first_call() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src/commands")).expect("source directory");
    fs::write(
        root.join("src/commands/stop.ts"),
        "export function stop(): never { process.exit(1); }\n",
    )
    .expect("source");

    let mut state = state(root);
    let params = json!({
        "name": explain_file::TOOL_NAME,
        "arguments": {
            "path": "src/commands/stop.ts",
            "rule": "language.javascript.runtime-exit-risk",
            "signal": "js.process-exit"
        }
    });

    let first = tool_text(handle_tools_call(json!(7), &params, &mut state));
    let first_value: Value = serde_json::from_str(&first).expect("first explain JSON");
    assert_eq!(first_value["decision"]["final_severity"], "HIGH");

    fs::write(
        root.join("package.json"),
        r#"{
  "name": "fixture-cli",
  "bin": {
    "fixture-cli": "src/bin.ts"
  }
}
"#,
    )
    .expect("package manifest");

    let second = tool_text(handle_tools_call(json!(8), &params, &mut state));
    let second_value: Value = serde_json::from_str(&second).expect("second explain JSON");

    assert_eq!(second_value["decision"]["final_severity"], "LOW");
    assert!(
        second_value["context"]["roles"]
            .as_array()
            .is_some_and(|roles| roles.iter().any(|role| role == "cli-executable"))
    );
}
