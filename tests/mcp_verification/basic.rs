use super::*;

#[test]
fn absent_and_empty_selection_skip_consent_while_duplicate_selection_is_skipped_without_capability()
{
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf x >> verification-runs"]
"#,
    );
    let responses = run_mcp(
        temp.path(),
        vec![
            tool_call(1, json!({ "path": ".", "detail": "full" })),
            tool_call(2, json!({ "path": ".", "detail": "full", "verify": [] })),
            tool_call(
                3,
                json!({
                    "path": ".",
                    "detail": "full",
                    "verify": ["unit", "unit"]
                }),
            ),
        ],
    );

    for id in [1, 2] {
        let result = result_for(&responses, id);
        assert_eq!(result["isError"], false);
        assert!(
            result["structuredContent"]["merge_readiness"]
                .get("verification")
                .is_none()
        );
    }
    let selected = result_for(&responses, 3);
    assert_eq!(
        selected["structuredContent"]["merge_readiness"]["verification"]
            .as_array()
            .expect("verification outcomes")
            .len(),
        1
    );
    assert_eq!(
        selected["structuredContent"]["merge_readiness"]["verification"][0]["status"],
        "skipped"
    );
    assert_eq!(
        selected["structuredContent"]["merge_readiness"]["verification"][0]["limitations"][0],
        "client does not support approval"
    );
    assert!(!temp.path().join("verification-runs").exists());
}

#[test]
fn unknown_selector_fails_before_spawn_and_publication() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "known"
role = "test"
program = "sh"
args = ["-c", "printf spawned > should-not-exist"]
"#,
    );
    let responses = run_mcp(
        temp.path(),
        vec![
            tool_call(4, json!({ "path": ".", "verify": ["unknown"] })),
            json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "resources/read",
                "params": { "uri": "repopilot://analyses" }
            }),
        ],
    );

    let result = result_for(&responses, 4);
    assert_eq!(result["isError"], true);
    assert!(
        result["content"][0]["text"]
            .as_str()
            .expect("error")
            .contains("unknown verification check id")
    );
    assert!(!temp.path().join("should-not-exist").exists());
    assert_eq!(result_for(&responses, 5)["contents"][0]["text"], "[]");
}

#[test]
fn failed_check_is_blocked_publishable_evidence() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf failure >&2; exit 7"]
"#,
    );
    let mut client = client_with_form(temp.path());
    client.send(tool_call(
        6,
        json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
    ));
    let result = accept_prompt(&mut client, 6);
    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["merge_readiness"]["verification"][0]["status"],
        "failed"
    );
    assert_eq!(
        result["structuredContent"]["merge_readiness"]["verdict"],
        "blocked"
    );
    assert!(result["analysisHandle"].is_string());
    assert!(client.close().success());
}

#[test]
fn opted_in_cache_is_shared_by_mcp_calls_and_remains_publishable() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "mkdir -p .repopilot/cache; printf x >> .repopilot/cache/mcp-verification-runs"]
cache = { enabled = true }
"#,
    );
    let mut client = client_with_form(temp.path());
    client.send(tool_call(
        7,
        json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
    ));
    let first = accept_prompt(&mut client, 7);
    client.send(tool_call(
        8,
        json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
    ));
    let second = accept_prompt(&mut client, 8);
    assert!(
        first["structuredContent"]["merge_readiness"]["verification"][0]
            .get("reused")
            .is_none()
    );
    assert_eq!(
        second["structuredContent"]["merge_readiness"]["verification"][0]["reused"],
        true
    );
    assert!(first["analysisHandle"].is_string());
    assert!(second["analysisHandle"].is_string());
    assert_eq!(
        fs::read_to_string(temp.path().join(".repopilot/cache/mcp-verification-runs"))
            .expect("marker"),
        "x"
    );
    assert!(client.close().success());
}
