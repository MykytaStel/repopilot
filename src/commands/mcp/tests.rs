use super::{ServerState, handle_tools_call, scan, serve};
use serde_json::{Value, json};
use std::io::Cursor;

/// Runs the server over newline-delimited request lines and returns the decoded
/// JSON responses in order.
fn exchange(requests: &[Value]) -> Vec<Value> {
    let input = requests
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");

    let mut output = Vec::new();
    serve(Cursor::new(input), &mut output).expect("serve over in-memory buffers");

    decode(output)
}

fn initialized_exchange(requests: &[Value]) -> Vec<Value> {
    let mut all = vec![
        json!({
            "jsonrpc": "2.0",
            "id": "initialize",
            "method": "initialize",
            "params": { "protocolVersion": "2025-11-25" }
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }),
    ];
    all.extend_from_slice(requests);
    let mut responses = exchange(&all);
    responses.remove(0);
    responses
}

fn decode(output: Vec<u8>) -> Vec<Value> {
    String::from_utf8(output)
        .expect("responses are utf-8")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("each response line is valid json"))
        .collect()
}

#[test]
fn initialize_reports_server_info_and_tools_capability() {
    let responses = exchange(&[
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "protocolVersion": "2025-06-18" }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": { "protocolVersion": "2024-11-05" }
        }),
    ]);

    assert_eq!(responses.len(), 2);
    let result = &responses[0]["result"];
    assert_eq!(result["serverInfo"]["name"], "repopilot");
    assert!(result["capabilities"]["tools"].is_object());
    // Unsupported client versions negotiate to the latest server version.
    assert_eq!(result["protocolVersion"], "2025-11-25");
    assert_eq!(responses[1]["result"]["protocolVersion"], "2024-11-05");
}

#[test]
fn tools_list_advertises_all_tools_with_schemas() {
    let responses = initialized_exchange(&[json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    })]);

    let tools = responses[0]["result"]["tools"]
        .as_array()
        .expect("tools array");

    let names: Vec<&str> = tools
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

    // Every advertised tool exposes an object input schema.
    for tool in tools {
        assert_eq!(
            tool["inputSchema"]["type"], "object",
            "tool {}",
            tool["name"]
        );
        assert!(tool["outputSchema"].is_object());
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
    }
}

#[test]
fn notifications_receive_no_response() {
    // A notification carries no `id`; the server must stay silent.
    let responses = exchange(&[json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    })]);

    assert!(responses.is_empty());
}

#[test]
fn unknown_method_returns_method_not_found() {
    let responses = initialized_exchange(&[json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "does/not/exist"
    })]);

    assert_eq!(responses[0]["error"]["code"], -32601);
}

#[test]
fn unknown_tool_returns_in_band_error_result() {
    let responses = initialized_exchange(&[json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": { "name": "repopilot_nope", "arguments": {} }
    })]);

    // Tool-level failures are reported as a successful response with isError.
    let result = &responses[0]["result"];
    assert_eq!(result["isError"], true);
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("unknown tool")
    );
}

#[test]
fn malformed_line_returns_parse_error_and_server_continues() {
    let input = "this is not json\n{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"ping\"}";
    let mut output = Vec::new();
    serve(Cursor::new(input), &mut output).expect("serve");

    let responses = decode(output);
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["error"]["code"], -32700);
    assert_eq!(responses[1]["id"], 9);
    assert!(responses[1]["result"].is_object());
}

#[test]
fn lists_resources_and_prompts() {
    let responses = initialized_exchange(&[
        json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}),
        json!({"jsonrpc":"2.0","id":2,"method":"prompts/list"}),
        json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"prompts/get",
            "params":{"name":"review-change"}
        }),
    ]);

    assert!(
        responses[0]["result"]["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|resource| resource["uri"] == "repopilot://rules")
    );
    assert!(
        responses[0]["result"]["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|resource| resource["uri"] == "repopilot://repository-summary")
    );
    assert_eq!(
        responses[1]["result"]["prompts"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        responses[2]["result"]["messages"][0]["content"]["type"],
        "text"
    );
}

#[test]
fn context_tool_returns_markdown_brief_matching_its_output_schema() {
    // `serve` roots at the canonicalized CWD (the crate root under `cargo test`),
    // so the relative fixture path resolves and stays within the MCP root.
    let responses = initialized_exchange(&[json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": {
            "name": "repopilot_context",
            "arguments": { "path": "tests/fixtures/projects/ai-context-sample" }
        }
    })]);

    let result = &responses[0]["result"];
    assert_eq!(
        result["isError"],
        json!(false),
        "context call should succeed: {result}"
    );

    // The tool declares `outputSchema: { markdown: string }`. The structured
    // result must match that shape exactly — a single `markdown` string field.
    let structured = result["structuredContent"]
        .as_object()
        .expect("structuredContent is an object");
    assert_eq!(
        structured.keys().collect::<Vec<_>>(),
        vec!["markdown"],
        "structuredContent must be exactly {{ markdown }}: {structured:?}"
    );
    let markdown = structured["markdown"]
        .as_str()
        .expect("markdown is a string");
    assert!(
        markdown.contains("RepoPilot AI Context"),
        "markdown should be the AI context brief: {markdown}"
    );
    assert!(
        markdown.contains("DEBUG = True in Django settings"),
        "brief should surface the sample project's security finding"
    );

    // The text content block mirrors the same brief.
    assert_eq!(result["content"][0]["type"], json!("text"));
    assert_eq!(
        result["content"][0]["text"].as_str(),
        Some(markdown),
        "content text should mirror the structured markdown"
    );
}

#[test]
fn explain_finding_requires_a_session_result() {
    let responses = initialized_exchange(&[json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "tools/call",
        "params": {
            "name": "repopilot_explain_finding",
            "arguments": {
                "finding_id": "language.rust.panic-risk:src/lib.rs:1"
            }
        }
    })]);

    let result = &responses[0]["result"];
    assert_eq!(result["isError"], true);
    assert!(
        result["content"][0]["text"]
            .as_str()
            .is_some_and(|message| message.contains("run repopilot_scan first"))
    );
}

#[test]
fn scan_then_explain_finding_replays_session_finding() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source_path = temp.path().join("src/lib.rs");
    std::fs::create_dir_all(source_path.parent().expect("parent"))
        .expect("create source directory");
    std::fs::write(&source_path, "pub fn dangerous() { panic!(\"boom\"); }\n")
        .expect("write Rust source");

    let scan_report = scan::call(&json!({
        "path": temp.path(),
        "profile": "strict"
    }))
    .expect("scan fixture repository");
    let scan_value: Value = serde_json::from_str(&scan_report).expect("scan report JSON");
    let finding_id = scan_value["findings"]
        .as_array()
        .expect("findings array")
        .iter()
        .find(|finding| {
            finding["rule_id"] == "language.rust.panic-risk"
                && finding["provenance"]["analysis_scope"] == "file"
                && finding["provenance"]["knowledge_decision"].is_object()
        })
        .and_then(|finding| finding["id"].as_str())
        .expect("knowledge-aware Rust panic finding")
        .to_string();

    let mut state = ServerState {
        root: temp.path().canonicalize().expect("canonical root"),
        initialized: true,
        last_scan: Some(scan_report),
        ..ServerState::default()
    };
    let response = handle_tools_call(
        json!(9),
        &json!({
            "name": "repopilot_explain_finding",
            "arguments": {
                "finding_id": finding_id,
                "source": "last-scan"
            }
        }),
        &mut state,
    );
    let result = response.result.expect("MCP result");

    assert_eq!(result["isError"], false, "tool call failed: {result}");
    assert_eq!(result["structuredContent"]["source_report"], "last-scan");
    assert_eq!(result["structuredContent"]["replay"]["status"], "matched");
    assert!(
        result["structuredContent"]["explanation"]["decision"]["trace"]
            .as_array()
            .is_some_and(|trace| !trace.is_empty())
    );
}
