//! End-to-end test for the `repopilot mcp` stdio server: spawn the real binary,
//! drive it with JSON-RPC over stdin, and assert the tool surface and a local
//! tool call. The whole exchange runs offline from on-disk files, exercising the
//! local-first promise (no network, no AI service).

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

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

fn start_mcp(cwd: &Path) -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .arg("mcp")
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn `repopilot mcp`");
    let stdin = child.stdin.take().expect("child stdin");
    let stdout = BufReader::new(child.stdout.take().expect("child stdout"));
    (child, stdin, stdout)
}

fn send_mcp(stdin: &mut ChildStdin, request: &Value) {
    writeln!(stdin, "{request}").expect("write MCP request");
    stdin.flush().expect("flush MCP request");
}

fn receive_mcp(stdout: &mut BufReader<ChildStdout>) -> Value {
    let mut line = String::new();
    stdout.read_line(&mut line).expect("read MCP response");
    assert!(!line.is_empty(), "MCP server closed before responding");
    serde_json::from_str(&line).expect("MCP response JSON")
}

fn initialize_mcp(stdin: &mut ChildStdin, stdout: &mut BufReader<ChildStdout>) {
    send_mcp(
        stdin,
        &json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-11-25"}}),
    );
    let _initialize = receive_mcp(stdout);
    send_mcp(
        stdin,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    );
}

#[test]
fn mcp_server_rejects_non_2_jsonrpc_envelopes() {
    let temp = tempfile::tempdir().expect("temp dir");
    let responses = run_mcp(
        &[r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#],
        temp.path(),
    );

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["id"], Value::Null);
    assert_eq!(responses[0]["error"]["code"], -32600);
}

fn setup_removed_export_change(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    fs::create_dir_all(root.join("src")).expect("src dir");
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
        .expect("removed-export signal")
}

fn assert_removed_export_explanation(explanation: &Value, signal_id: &str) {
    assert_eq!(explanation["signal"]["signal_id"], signal_id);
    assert_eq!(explanation["signal"]["path"], "src/caller.ts");
    assert_eq!(explanation["signal"]["target_path"], "src/api.ts");
    assert_eq!(explanation["impact"]["path"], "src/api.ts");
    assert_eq!(
        explanation["impact"]["direct_dependents"][0],
        "src/caller.ts"
    );
    assert_eq!(explanation["gate"]["eligible"], true);
    assert!(
        explanation["verification_plan"]["steps"]
            .as_array()
            .is_some_and(|steps| !steps.is_empty())
    );
    assert_eq!(explanation["signal"]["provenance"]["signal_source"], "ast");
    assert!(
        explanation["limitations"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert_eq!(
        explanation["why_it_matters"],
        "removed export is still imported. The caller imports a named symbol that the changed module no longer exports, which can break that import contract. This is static Git-diff evidence; RepoPilot does not execute the compiler or claim full module-resolution parity."
    );
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
            "repopilot_explain_review_signal",
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
    assert!(report["health_score"].is_u64());
    assert!(report["maintainability_score"].is_u64());
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
fn mcp_review_projects_the_canonical_merge_readiness_record() {
    let temp = tempfile::tempdir().expect("temp dir");
    fs::create_dir_all(temp.path().join("src")).expect("src dir");
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn value() -> i32 { 1 }\n",
    )
    .unwrap();
    git(temp.path(), &["init", "-q"]);
    git(temp.path(), &["config", "user.email", "test@example.com"]);
    git(temp.path(), &["config", "user.name", "Test"]);
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-qm", "initial"]);
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn value() -> i32 { 2 }\n",
    )
    .unwrap();

    let responses = run_mcp(
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"repopilot_review_change","arguments":{"path":"."}}}"#,
        ],
        temp.path(),
    );

    let text = responses[1]["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    let report: Value = serde_json::from_str(text).expect("review report is json");
    assert!(matches!(
        report["merge_readiness"]["verdict"].as_str(),
        Some("ready" | "review" | "blocked")
    ));
    assert!(report["merge_readiness"]["impact"].is_object());
    assert!(report["merge_readiness"]["ownership"].is_object());
}

#[test]
fn mcp_review_and_explanation_share_the_removed_export_occurrence() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path();
    setup_removed_export_change(root);

    let (mut child, mut stdin, mut stdout) = start_mcp(root);
    initialize_mcp(&mut stdin, &mut stdout);
    send_mcp(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":"review","method":"tools/call","params":{"name":"repopilot_review_change","arguments":{"path":".","detail":"full","fail_on_review":"definitely"}}}),
    );
    let review = receive_mcp(&mut stdout);
    let report = &review["result"]["structuredContent"];
    let signal = removed_export_signal(report);
    assert_eq!(signal["path"], "src/caller.ts");
    assert_eq!(signal["target_path"], "src/api.ts");
    assert_eq!(signal["gate_eligible"], true);
    assert_eq!(signal["provenance"]["detector"], signal["kind"]);
    assert_eq!(report["review_gate"]["failed_signals"], 1);
    let signal_id = signal["signal_id"].as_str().expect("signal id");

    send_mcp(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":"explain","method":"tools/call","params":{"name":"repopilot_explain_review_signal","arguments":{"signal_id":signal_id}}}),
    );
    let explain = receive_mcp(&mut stdout);
    let explanation = &explain["result"]["structuredContent"];
    assert_removed_export_explanation(explanation, signal_id);

    drop(stdin);
    assert!(child.wait().expect("wait for MCP server").success());
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

#[cfg(unix)]
#[test]
fn mcp_review_emits_check_aware_redaction_safe_progress() {
    let temp = tempfile::tempdir().expect("temp dir");
    git(temp.path(), &["init", "-q"]);
    git(temp.path(), &["config", "user.email", "test@example.com"]);
    git(temp.path(), &["config", "user.name", "Test"]);
    fs::write(
        temp.path().join("lib.rs"),
        "pub fn value() -> usize { 1 }\n",
    )
    .expect("source");
    fs::write(
        temp.path().join("repopilot.toml"),
        r#"[[verification.checks]]
id = "lint"
role = "lint"
program = "sh"
args = ["-c", "printf 'token=progress-secret'; exit 0"]
[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf 'token=progress-secret' >&2; exit 7"]
"#,
    )
    .expect("config");
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-qm", "initial"]);
    fs::write(
        temp.path().join("lib.rs"),
        "pub fn value() -> usize { 2 }\n",
    )
    .expect("change");

    let responses = run_mcp(
        &[
            r#"{"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"repopilot_review_change","arguments":{"path":".","verify":["unit","lint"]},"_meta":{"progressToken":"review-8"}}}"#,
        ],
        temp.path(),
    );
    let progress = responses
        .iter()
        .filter(|message| message["method"] == "notifications/progress")
        .collect::<Vec<_>>();
    let sequence = progress
        .iter()
        .map(|message| {
            (
                message["params"]["progress"].as_u64().expect("progress"),
                message["params"]["total"].as_u64().expect("total"),
                message["params"]["message"]
                    .as_str()
                    .expect("message")
                    .to_string(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        sequence,
        vec![
            (0, 4, "analysis started".to_string()),
            (1, 4, "analysis complete".to_string()),
            (1, 4, "verification lint started".to_string()),
            (2, 4, "verification lint passed".to_string()),
            (2, 4, "verification unit started".to_string()),
            (3, 4, "verification unit failed".to_string()),
            (4, 4, "review complete".to_string()),
        ]
    );
    let encoded = serde_json::to_string(&responses).expect("responses");
    assert!(!encoded.contains("progress-secret"));
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

#[cfg(unix)]
fn setup_slow_verification_repo(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    fs::write(root.join("lib.rs"), "pub fn value() -> usize { 1 }\n").expect("source");
    fs::write(
        root.join("repopilot.toml"),
        r#"[[verification.checks]]
id = "slow"
role = "test"
program = "sh"
args = ["-c", "printf started > verification-started; sleep 30"]
timeout_seconds = 2
"#,
    )
    .expect("config");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "initial"]);
    fs::write(root.join("lib.rs"), "pub fn value() -> usize { 2 }\n").expect("change");
}

#[cfg(unix)]
fn slow_verification_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("slow verification test lock")
}

#[cfg(unix)]
#[test]
fn mcp_cancellation_stops_active_verification() {
    let _guard = slow_verification_guard();
    let temp = tempfile::tempdir().expect("temp dir");
    setup_slow_verification_repo(temp.path());

    let (mut child, mut stdin, mut stdout) = start_mcp(temp.path());
    initialize_mcp(&mut stdin, &mut stdout);
    send_mcp(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 17,
            "method": "tools/call",
            "params": {
                "name": "repopilot_review_change",
                "arguments": { "path": ".", "verify": ["slow"] }
            }
        }),
    );
    let marker = temp.path().join("verification-started");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !marker.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(marker.exists(), "verification process did not start");

    let cancelled_at = std::time::Instant::now();
    send_mcp(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": { "requestId": 17, "reason": "test active cancellation" }
        }),
    );
    let response = receive_mcp(&mut stdout);
    let cancellation_latency = cancelled_at.elapsed();
    drop(stdin);
    let status = child.wait().expect("MCP server exits");

    assert!(status.success());
    assert_eq!(response["id"], 17);
    assert_eq!(response["error"]["code"], -32800);
    assert!(
        cancellation_latency < std::time::Duration::from_secs(1),
        "active cancellation took {cancellation_latency:?}"
    );
}

#[cfg(unix)]
#[test]
fn mcp_queue_overload_stays_responsive_during_an_active_tool() {
    let _guard = slow_verification_guard();
    let temp = tempfile::tempdir().expect("temp dir");
    setup_slow_verification_repo(temp.path());
    let (mut child, mut stdin, mut stdout) = start_mcp(temp.path());
    initialize_mcp(&mut stdin, &mut stdout);
    send_mcp(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 40,
            "method": "tools/call",
            "params": {
                "name": "repopilot_review_change",
                "arguments": { "path": ".", "verify": ["slow"] }
            }
        }),
    );
    let marker = temp.path().join("verification-started");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !marker.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(marker.exists(), "verification process did not start");

    let overloaded_at = std::time::Instant::now();
    for id in 100..=108 {
        send_mcp(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": { "name": "repopilot_nope", "arguments": {} }
            }),
        );
    }
    let overloaded = receive_mcp(&mut stdout);
    assert_eq!(overloaded["id"], 108);
    assert_eq!(overloaded["error"]["code"], -32000);
    assert!(
        overloaded_at.elapsed() < std::time::Duration::from_secs(1),
        "overload response was blocked behind active analysis"
    );

    let cancelled_at = std::time::Instant::now();
    send_mcp(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": { "requestId": 40, "reason": "test overload cancellation" }
        }),
    );
    let cancelled = receive_mcp(&mut stdout);
    assert_eq!(cancelled["id"], 40);
    assert_eq!(cancelled["error"]["code"], -32800);
    assert!(
        cancelled_at.elapsed() < std::time::Duration::from_secs(1),
        "cancellation was blocked behind queued work"
    );

    drop(stdin);
    assert!(child.wait().expect("MCP server exits").success());
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
