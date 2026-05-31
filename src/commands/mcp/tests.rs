use super::serve;
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
    let responses = exchange(&[json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": { "protocolVersion": "2025-06-18" }
    })]);

    assert_eq!(responses.len(), 1);
    let result = &responses[0]["result"];
    assert_eq!(result["serverInfo"]["name"], "repopilot");
    assert!(result["capabilities"]["tools"].is_object());
    // The client's requested protocol version is echoed back.
    assert_eq!(result["protocolVersion"], "2025-06-18");
}

#[test]
fn tools_list_advertises_all_tools_with_schemas() {
    let responses = exchange(&[json!({
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
        ]
    );

    // Every advertised tool exposes an object input schema.
    for tool in tools {
        assert_eq!(
            tool["inputSchema"]["type"], "object",
            "tool {}",
            tool["name"]
        );
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
    let responses = exchange(&[json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "does/not/exist"
    })]);

    assert_eq!(responses[0]["error"]["code"], -32601);
}

#[test]
fn unknown_tool_returns_in_band_error_result() {
    let responses = exchange(&[json!({
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
fn malformed_line_is_skipped_and_server_continues() {
    // A non-JSON line must be skipped without tearing down the server, which
    // then answers the following valid request.
    let input = "this is not json\n{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"ping\"}";
    let mut output = Vec::new();
    serve(Cursor::new(input), &mut output).expect("serve");

    let responses = decode(output);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["id"], 9);
    assert!(responses[0]["result"].is_object());
}
