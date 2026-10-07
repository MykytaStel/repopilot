use super::*;

#[test]
fn wrong_elicitation_response_id_is_ignored_and_matching_accept_runs_check() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf x >> verification-runs"]
"#,
    );
    let mut client = InteractiveMcpClient::start(
        temp.path(),
        "2025-11-25",
        json!({
            "elicitation": { "form": {} }
        }),
    );
    client.send(json!({
        "jsonrpc": "2.0",
        "id": 70,
        "method": "tools/call",
        "params": {
            "name": "repopilot_review_change",
            "arguments": { "path": ".", "detail": "full", "verify": ["unit"] }
        }
    }));

    let prompt = client.receive();
    assert_eq!(
        prompt["method"], "elicitation/create",
        "MCP verification must wait for a check-specific consent request"
    );
    assert_eq!(prompt["params"]["mode"], "form");
    assert_eq!(
        prompt["params"]["requestedSchema"]["required"][0],
        "approve"
    );
    let message = prompt["params"]["message"].as_str().unwrap();
    assert!(message.contains("Program: /") || message.contains("Program: sh"));
    assert!(
        prompt["params"]["message"]
            .as_str()
            .unwrap()
            .contains("Arguments (exact)")
    );
    assert!(
        prompt["params"]["message"]
            .as_str()
            .unwrap()
            .contains("not sandboxed")
    );
    assert!(
        !prompt["params"]["message"]
            .as_str()
            .unwrap()
            .contains("HOME=")
    );
    let request_id = prompt["id"].clone();
    client.send(json!({
        "jsonrpc": "2.0",
        "id": "repopilot/elicitation/wrong",
        "result": { "action": "accept", "content": { "approve": true } }
    }));
    client.send(json!({ "jsonrpc": "2.0", "id": 71, "method": "ping" }));
    assert_eq!(client.receive()["id"], 71, "wrong response was consumed");
    client.send(json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "result": { "action": "accept", "content": { "approve": true } }
    }));

    let response = client.receive_with_id(70);
    let outcome = &response["result"]["structuredContent"]["merge_readiness"]["verification"][0];
    assert_eq!(outcome["status"], "passed");
    assert_eq!(
        fs::read_to_string(temp.path().join("verification-runs")).expect("marker"),
        "x"
    );
    assert!(client.close().success());
}

#[test]
fn unsupported_elicitation_negotiation_skips_without_prompt_or_spawn() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf x >> verification-runs"]
"#,
    );
    for (protocol, capabilities) in [
        ("2025-11-25", json!({})),
        ("2024-11-05", json!({ "elicitation": { "form": {} } })),
    ] {
        let mut client = InteractiveMcpClient::start(temp.path(), protocol, capabilities);
        client.send(tool_call(
            80,
            json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
        ));
        let response = client.receive();
        assert_eq!(
            response["id"], 80,
            "unsupported consent must emit no prompt"
        );
        let outcome =
            &response["result"]["structuredContent"]["merge_readiness"]["verification"][0];
        assert_eq!(outcome["status"], "skipped");
        assert_eq!(
            outcome["limitations"][0],
            "client does not support approval"
        );
        assert!(!temp.path().join("verification-runs").exists());
        assert!(client.close().success());
    }
}

#[test]
fn capable_client_with_empty_selection_receives_no_elicitation() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf x >> verification-runs"]
"#,
    );
    let mut client = client_with_form(temp.path());
    client.send(tool_call(
        90,
        json!({ "path": ".", "detail": "full", "verify": [] }),
    ));
    let response = client.receive();
    assert_eq!(response["id"], 90, "empty selection must emit no prompt");
    assert_eq!(response["result"]["isError"], false);
    assert!(
        response["result"]["structuredContent"]["merge_readiness"]
            .get("verification")
            .is_none()
    );
    assert!(!temp.path().join("verification-runs").exists());
    assert!(client.close().success());
}

#[test]
fn unresolved_executable_skips_without_elicitation() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "repopilot-intentionally-missing-executable-7c9c3f"
args = []
"#,
    );
    let mut client = client_with_form(temp.path());
    client.send(tool_call(
        92,
        json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
    ));
    let response = client.receive();
    assert_eq!(
        response["id"], 92,
        "unresolved executable must emit no prompt"
    );
    let outcome = &response["result"]["structuredContent"]["merge_readiness"]["verification"][0];
    assert_eq!(outcome["status"], "skipped");
    assert_eq!(
        outcome["limitations"][0],
        "configured executable could not be resolved"
    );
    assert!(!temp.path().join("verification-runs").exists());
    assert!(client.close().success());
}

#[test]
fn elicitation_cancel_skips_without_launching_check() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf x >> verification-runs"]
"#,
    );
    let mut client = client_with_form(temp.path());
    client.send(tool_call(
        81,
        json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
    ));
    let prompt = client.receive();
    assert_eq!(prompt["method"], "elicitation/create");
    client.send(json!({
        "jsonrpc": "2.0",
        "id": prompt["id"],
        "result": { "action": "cancel" }
    }));
    let response = client.receive_with_id(81);
    let outcome = &response["result"]["structuredContent"]["merge_readiness"]["verification"][0];
    assert_eq!(outcome["status"], "skipped");
    assert_eq!(outcome["limitations"][0], "user cancelled");
    assert!(!temp.path().join("verification-runs").exists());
    assert!(client.close().success());
}

#[test]
fn malformed_approval_and_jsonrpc_error_fail_closed() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf x >> verification-runs"]
"#,
    );
    let mut client = client_with_form(temp.path());
    for (call_id, response) in [
        (
            84,
            json!({ "action": "accept", "content": { "approve": false } }),
        ),
        (
            86,
            json!({ "action": "accept", "content": { "approve": true, "extra": 1 } }),
        ),
        (
            85,
            json!({ "error": { "code": -32603, "message": "client error" } }),
        ),
    ] {
        client.send(tool_call(
            call_id,
            json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
        ));
        let prompt = client.receive();
        assert_eq!(prompt["method"], "elicitation/create");
        let mut reply = json!({ "jsonrpc": "2.0", "id": prompt["id"] });
        if response.get("error").is_some() {
            reply["error"] = response["error"].clone();
        } else {
            reply["result"] = response;
        }
        client.send(reply);
        let outcome = client.receive_with_id(call_id)["result"]["structuredContent"]["merge_readiness"]["verification"][0].clone();
        assert_eq!(outcome["status"], "skipped");
        assert_eq!(outcome["limitations"][0], "invalid approval response");
    }
    assert!(!temp.path().join("verification-runs").exists());
    assert!(client.close().success());
}
