//! End-to-end test for the `repopilot mcp` stdio server: spawn the real binary,
//! drive it with JSON-RPC over stdin, and assert the tool surface and a local
//! tool call. The whole exchange runs offline from on-disk files, exercising the
//! local-first promise (no network, no AI service).

use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Runs `repopilot mcp` over `requests` (one JSON-RPC message each) and returns
/// the decoded responses in order. Closing stdin ends the server loop.
fn run_mcp(requests: &[&str], cwd: &Path) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .arg("mcp")
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn `repopilot mcp`");

    {
        let mut stdin = child.stdin.take().expect("child stdin");
        for request in requests {
            stdin.write_all(request.as_bytes()).expect("write request");
            stdin.write_all(b"\n").expect("write newline");
        }
        // Dropping stdin sends EOF, which ends the server's read loop.
    }

    let output = child.wait_with_output().expect("wait for `repopilot mcp`");
    assert!(
        output.status.success(),
        "server exited with {:?}",
        output.status.code()
    );

    String::from_utf8(output.stdout)
        .expect("stdout is utf-8")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("response line is json"))
        .collect()
}

#[test]
fn mcp_server_initializes_lists_tools_and_runs_scan_locally() {
    let temp = tempfile::tempdir().expect("temp dir");
    fs::create_dir_all(temp.path().join("src")).expect("src dir");
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .expect("source file");
    fs::write(
        temp.path().join("src/config.ts"),
        "export const API_KEY = \"abc123xyz987\";\n",
    )
    .expect("secret fixture");

    let responses = run_mcp(
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"repopilot_scan","arguments":{"path":".","profile":"strict","filters":{"rules":["security.secret-candidate"]}}}}"#,
        ],
        temp.path(),
    );

    assert_eq!(responses.len(), 3, "one response per request");

    // initialize advertises the server identity.
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "repopilot");

    // tools/list exposes the full tool surface in order.
    let names: Vec<&str> = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect();
    assert_eq!(
        names,
        [
            "repopilot_review_change",
            "repopilot_scan",
            "repopilot_context",
            "repopilot_explain_file",
            "repopilot_explain_finding",
        ]
    );

    // tools/call runs the scan entirely from local files and returns a JSON report.
    let result = &responses[2]["result"];
    assert_eq!(result["isError"], false, "scan should succeed");
    assert!(result["structuredContent"].is_object());
    let text = result["content"][0]["text"].as_str().expect("text content");
    let report: Value = serde_json::from_str(text).expect("scan report is json");
    assert!(
        report["schema_version"].is_string(),
        "scan report carries schema metadata"
    );
    assert!(report["report"].is_object(), "scan report carries findings");
    let findings = report["findings"].as_array().expect("findings");
    assert!(!findings.is_empty());
    assert!(
        findings
            .iter()
            .all(|finding| finding["rule_id"] == "security.secret-candidate")
    );
}

#[test]
fn mcp_context_tool_includes_repository_facts() {
    // Phase 1: the context tool wraps the facts-aware renderer (like the CLI),
    // so an agent gets the aggregate stack/size picture, not a thinner brief.
    let temp = tempfile::tempdir().expect("temp dir");
    fs::create_dir_all(temp.path().join("src")).expect("src dir");
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn live() -> i32 {\n    1\n}\n",
    )
    .expect("source file");

    let responses = run_mcp(
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"repopilot_context","arguments":{"path":"."}}}"#,
        ],
        temp.path(),
    );

    let result = &responses[1]["result"];
    assert_eq!(result["isError"], false, "context should succeed");
    assert!(
        responses[1]
            .to_string()
            .contains("Repository Facts Summary"),
        "context tool should include the repository facts section: {}",
        responses[1]
    );
}

#[test]
fn mcp_server_reports_unknown_tool_as_in_band_error() {
    let temp = tempfile::tempdir().expect("temp dir");

    let responses = run_mcp(
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nope","arguments":{}}}"#,
        ],
        temp.path(),
    );

    let result = &responses[1]["result"];
    assert_eq!(result["isError"], true);
    assert!(
        result["content"][0]["text"]
            .as_str()
            .expect("text")
            .contains("unknown tool")
    );
}

#[test]
fn mcp_server_rejects_paths_outside_workspace_root() {
    let root = tempfile::tempdir().expect("root temp dir");
    let outside = tempfile::tempdir().expect("outside temp dir");
    let request = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"repopilot_scan","arguments":{{"path":"{}"}}}}}}"#,
        outside.path().display()
    );

    let responses = run_mcp(&[&request], root.path());
    assert_eq!(responses[0]["error"]["code"], -32002);

    let initialized = run_mcp(
        &[
            r#"{"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            &request,
        ],
        root.path(),
    );
    let result = &initialized[1]["result"];
    assert_eq!(result["isError"], true);
    assert!(
        result["content"][0]["text"]
            .as_str()
            .expect("text")
            .contains("must stay within MCP root")
    );
}

#[test]
fn mcp_server_emits_progress_for_tool_calls() {
    let temp = tempfile::tempdir().expect("temp dir");
    fs::write(temp.path().join("lib.rs"), "pub fn live() {}\n").expect("source");

    let responses = run_mcp(
        &[
            r#"{"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"repopilot_scan","arguments":{"path":"."},"_meta":{"progressToken":"scan-7"}}}"#,
        ],
        temp.path(),
    );

    let progress = responses
        .iter()
        .filter(|message| message["method"] == "notifications/progress")
        .collect::<Vec<_>>();
    assert_eq!(progress.len(), 2);
    assert_eq!(progress[0]["params"]["progressToken"], "scan-7");
    assert_eq!(progress[0]["params"]["progress"], 0);
    assert_eq!(progress[1]["params"]["progress"], 1);
    assert!(responses.iter().any(|message| message["id"] == 7));
}

#[test]
fn mcp_server_cancels_background_tool_calls() {
    let temp = tempfile::tempdir().expect("temp dir");
    fs::create_dir(temp.path().join("src")).expect("src");
    for index in 0..200 {
        fs::write(
            temp.path().join("src").join(format!("module{index}.rs")),
            format!("pub fn value_{index}() -> usize {{ {index} }}\n"),
        )
        .expect("source");
    }

    let responses = run_mcp(
        &[
            r#"{"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"repopilot_scan","arguments":{"path":"."}}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":9,"reason":"test cancellation"}}"#,
        ],
        temp.path(),
    );

    let cancelled = responses
        .iter()
        .find(|message| message["id"] == 9)
        .expect("cancelled response");
    assert_eq!(cancelled["error"]["code"], -32800);
}

fn git(root: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git available")
        .status
        .success();
    assert!(ok, "git {args:?} failed");
}

fn git_path(root: &Path, path: &str) -> PathBuf {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--git-path", path])
        .output()
        .expect("git available");
    assert!(output.status.success(), "git rev-parse --git-path failed");
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

#[test]
fn mcp_scan_cache_persists_across_sessions_and_invalidates_on_edit() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "t@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(root.join("src/lib.rs"), "pub fn live() -> i32 { 1 }\n").expect("source");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "init"]);

    let scan = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"repopilot_scan","arguments":{"path":"."}}}"#,
    ];

    // Session 1 writes the disk cache.
    let first = run_mcp(&scan, root);
    assert_eq!(first[1]["result"]["isError"], false, "scan should succeed");
    let cache_dir = git_path(root, "repopilot/cache/mcp-scan");
    let cache_file = fs::read_dir(&cache_dir)
        .expect("cache dir created")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "json"))
        .expect("a cache entry was written");

    // Overwrite the cache with a sentinel: a NEW process scanning the unchanged
    // tree must read it back — proving a cross-session disk hit.
    fs::write(
        &cache_file,
        r#"{"schema_version":"test","report":{"kind":"scan"},"sentinel":"cached-across-sessions"}"#,
    )
    .expect("overwrite cache");
    let hit = run_mcp(&scan, root);
    assert!(
        hit[1].to_string().contains("cached-across-sessions"),
        "a second session must serve the disk cache: {}",
        hit[1]
    );

    // Editing a file changes the working-tree fingerprint → miss → a real scan,
    // never the stale sentinel.
    fs::write(root.join("src/lib.rs"), "pub fn changed() -> i32 { 2 }\n").expect("edit");
    let miss = run_mcp(&scan, root);
    let text = miss[1].to_string();
    assert!(
        !text.contains("cached-across-sessions"),
        "an edit must invalidate the cache"
    );
    assert!(
        text.contains("schema_version"),
        "a miss returns a real scan report"
    );
}
