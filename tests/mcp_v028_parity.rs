use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn mcp_matches_python_and_go_cross_file_integrity_signals() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    prepare_integrity_fixture(root);
    assert_integrity_cli_mcp_parity(root);
}

fn prepare_integrity_fixture(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("src dir");
    fs::create_dir_all(root.join("tests")).expect("tests dir");
    write(
        root,
        "src/checks.py",
        "def verify(value):\n    assert value\n    assert value is not None\n",
    );
    write(
        root,
        "tests/test_checks.py",
        "from src.checks import verify as check\n\ndef test_request():\n    check(1)\n",
    );
    write(
        root,
        "src/verify.go",
        "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Error(\"bad\"); t.Fatal(\"stop\") }\n",
    );
    write(
        root,
        "src/verify_test.go",
        "package app\nimport \"testing\"\nfunc TestRequest(t *testing.T) { Verify(t) }\n",
    );
    init_repo(root);
    write(
        root,
        "src/checks.py",
        "def verify(value):\n    assert value\n",
    );
    write(
        root,
        "src/verify.go",
        "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Error(\"bad\") }\n",
    );
}

fn assert_integrity_cli_mcp_parity(root: &Path) {
    let cli = cli_review(root);
    let mcp = mcp_review(root);
    let cli_signals = integrity_signals(&cli);
    assert_eq!(cli_signals.len(), 2, "signals: {cli_signals:?}");
    assert!(cli_signals.iter().any(|signal| {
        signal["path"] == "tests/test_checks.py" && signal["kind"] == "integrity.assertions-removed"
    }));
    assert!(cli_signals.iter().any(|signal| {
        signal["path"] == "src/verify_test.go" && signal["kind"] == "integrity.assertions-removed"
    }));
    assert_eq!(integrity_signals(&mcp), cli_signals);
    assert_eq!(
        mcp["change_proof"]["verdict"],
        cli["change_proof"]["verdict"]
    );
}

#[test]
fn mcp_matches_removed_rust_enum_variant_contract_evidence() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("src dir");
    write(root, "src/api.rs", "pub enum State { Old, Ready }\n");
    write(
        root,
        "src/lib.rs",
        "mod api;\nuse crate::api::State as Status;\nfn run() { let _ = Status::Old; }\n",
    );
    init_repo(root);
    write(root, "src/api.rs", "pub enum State { Ready }\n");

    let cli = cli_review(root);
    let mcp = mcp_review(root);
    let cli_signals = enum_variant_signals(&cli);
    assert_eq!(cli_signals.len(), 1, "signals: {cli_signals:?}");
    assert_eq!(enum_variant_signals(&mcp), cli_signals);
    assert_eq!(cli_signals[0]["path"], "src/lib.rs");
    let deltas = &cli["change_proof"]["contract_deltas"];
    assert_eq!(mcp["change_proof"]["contract_deltas"], *deltas);
    assert!(deltas.as_array().unwrap().iter().any(|delta| {
        delta["family"] == "public-symbol" && delta["change"] == "removed-export"
    }));
    assert!(
        cli_signals[0]["detail"]
            .as_str()
            .unwrap()
            .contains("src/api.rs:1-1")
    );
}

#[test]
fn mcp_matches_taint_from_a_changed_call_to_an_unchanged_helper_sink() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root,
        "src/handler.ts",
        "function run_query(query: string) { db.query(\"SELECT * FROM users WHERE id = \" + query); }\nfunction handler() { run_query(\"safe\"); }\n",
    );
    init_repo(root);
    write(
        root,
        "src/handler.ts",
        "function run_query(query: string) { db.query(\"SELECT * FROM users WHERE id = \" + query); }\nfunction handler() { run_query(req.query.id); }\n",
    );

    let cli = cli_review(root);
    let mcp = mcp_review(root);
    let cli_taint = taint_signals(&cli);
    assert_eq!(cli_taint.len(), 1, "signals: {cli_taint:?}");
    assert_eq!(taint_signals(&mcp), cli_taint);
    assert_eq!(cli_taint[0]["path"], "src/handler.ts");
    assert!(cli_taint[0]["detail"].as_str().unwrap().contains("line 2"));
}

fn init_repo(root: &Path) {
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "test@example.com"],
        &["config", "user.name", "Test"],
        &["add", "."],
        &["commit", "-qm", "initial"],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }
}

fn cli_review(root: &Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["review", ".", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("run review");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("CLI JSON")
}

fn mcp_review(root: &Path) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .arg("mcp")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn MCP server");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        for request in [
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"repopilot_review_change","arguments":{"path":"."}}}"#,
        ] {
            writeln!(stdin, "{request}").expect("write MCP request");
        }
    }
    let output = child.wait_with_output().expect("wait for MCP server");
    let response = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|message| message["id"] == 2)
        .expect("tool response");
    response["result"]["structuredContent"].clone()
}

fn integrity_signals(report: &Value) -> Vec<Value> {
    signals(report)
        .into_iter()
        .filter(|signal| signal["family"] == "integrity")
        .collect()
}

fn enum_variant_signals(report: &Value) -> Vec<Value> {
    signals(report)
        .into_iter()
        .filter(|signal| signal["kind"] == "behavioral.rust-public-enum-variant-removed")
        .collect()
}

fn taint_signals(report: &Value) -> Vec<Value> {
    signals(report)
        .into_iter()
        .filter(|signal| signal["family"] == "taint")
        .collect()
}

fn signals(report: &Value) -> Vec<Value> {
    ["definitely", "maybe", "noise"]
        .iter()
        .flat_map(|tier| {
            report["tiered_signals"][tier]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .collect()
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent directory")).expect("create parent");
    fs::write(path, content).expect("write fixture");
}
