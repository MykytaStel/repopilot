use super::*;
use crate::commands::mcp::workspace_freshness_tests::{git, state};
use std::fs;
use std::path::Path;

#[test]
fn review_tool_reports_and_explains_a_rust_arity_break() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
    write(root, "src/api.rs", "pub fn load(user: usize) {}\n");
    write(root, "src/lib.rs", "mod api;\nfn run() { api::load(1); }\n");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "before arity change"]);
    write(
        root,
        "src/api.rs",
        "pub fn load(user: usize, mode: bool) {}\n",
    );
    let mut state = state(root);

    let response = handle_tools_call(
        json!(50),
        &json!({
            "name": "repopilot_review_change",
            "arguments": { "path": ".", "detail": "full" }
        }),
        &mut state,
    );
    let result = response.result.expect("review result");
    assert_eq!(result["isError"], false, "{result:#?}");
    let signals = result["structuredContent"]["tiered_signals"]["definitely"]
        .as_array()
        .expect("definitely signals");
    let signal = signals
        .iter()
        .find(|signal| signal["kind"] == "behavioral.rust-public-function-arity-changed")
        .expect("Rust arity signal");
    assert_eq!(signal["path"], "src/lib.rs");
    assert_eq!(signal["target_path"], "src/api.rs");
    let signal_id = signal["signal_id"].as_str().expect("signal id");

    let explain = handle_tools_call(
        json!(51),
        &json!({
            "name": "repopilot_explain_review_signal",
            "arguments": { "signal_id": signal_id }
        }),
        &mut state,
    );
    let explained = explain.result.expect("explain result");
    assert_eq!(explained["isError"], false, "{explained:#?}");
    assert!(
        explained["structuredContent"]["why_it_matters"]
            .as_str()
            .unwrap()
            .contains("still passes the previous number of arguments")
    );
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, content).expect("write source");
}
