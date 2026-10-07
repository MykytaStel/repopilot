use super::*;

#[test]
fn cancelling_tool_call_while_waiting_keeps_interleaved_ping_responsive() {
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
        86,
        json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
    ));
    let prompt = client.receive();
    assert_eq!(prompt["method"], "elicitation/create");
    client.send(json!({ "jsonrpc": "2.0", "id": 91, "method": "tools/list" }));
    assert_eq!(
        client.receive_with_id(91)["error"]["code"],
        -32000,
        "a concurrent request must not block the consent response"
    );
    client.send(json!({ "jsonrpc": "2.0", "method": "notifications/progress" }));
    client.send(json!({
        "jsonrpc": "2.0",
        "method": "notifications/cancelled",
        "params": { "requestId": 86 }
    }));
    client.send(json!({ "jsonrpc": "2.0", "id": 87, "method": "ping" }));
    assert_eq!(client.receive_with_id(87)["result"], json!({}));
    assert_eq!(client.receive_with_id(86)["error"]["code"], -32800);
    assert!(!temp.path().join("verification-runs").exists());
    assert!(client.close().success());
}

#[test]
fn workspace_and_explicit_config_drift_invalidate_pending_approval() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf x >> verification-runs"]
"#,
    );
    fs::write(temp.path().join(".gitignore"), "local.toml\n").expect("ignore local config");
    let original = r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf x >> verification-runs"]
"#;
    fs::write(temp.path().join("local.toml"), original).expect("explicit config");
    git(temp.path(), &["add", ".gitignore"]);
    git(temp.path(), &["commit", "-qm", "ignore local config"]);

    let mut client = client_with_form(temp.path());
    for (call_id, use_config) in [(88, false), (89, true)] {
        client.send(tool_call(
            call_id,
            json!({
                "path": ".",
                "detail": "full",
                "verify": ["unit"],
                "config": if use_config { "local.toml" } else { "repopilot.toml" }
            }),
        ));
        let prompt = client.receive();
        assert_eq!(prompt["method"], "elicitation/create");
        if use_config {
            let changed = original.replace("printf x", "printf changed");
            fs::write(temp.path().join("local.toml"), changed).expect("change ignored config");
        } else {
            fs::write(
                temp.path().join("lib.rs"),
                "pub fn value() -> usize { 3 }\n",
            )
            .expect("change workspace");
        }
        client.send(json!({
            "jsonrpc": "2.0",
            "id": prompt["id"],
            "result": { "action": "accept", "content": { "approve": true } }
        }));
        let outcome = client.receive_with_id(call_id)["result"]["structuredContent"]["merge_readiness"]["verification"][0].clone();
        assert_eq!(outcome["status"], "skipped");
        assert_eq!(
            outcome["limitations"][0],
            if use_config {
                "check definition changed before execution"
            } else {
                "workspace changed before execution"
            }
        );
    }
    assert!(!temp.path().join("verification-runs").exists());
    assert!(client.close().success());
}

#[test]
fn ignored_executable_replacement_while_approval_is_pending_invalidates_consent() {
    let temp = verification_repo(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = ".local/bin/check"
args = []
[verification.checks.cache]
enabled = true
"#,
    );
    fs::write(temp.path().join(".gitignore"), ".local/\n").expect("ignore local binary");
    git(temp.path(), &["add", ".gitignore"]);
    git(temp.path(), &["commit", "-qm", "ignore local binary"]);
    let executable = temp.path().join(".local/bin/check");
    fs::create_dir_all(executable.parent().expect("binary parent")).expect("binary directory");
    fs::write(
        &executable,
        "#!/bin/sh\nprintf original >> .local/verification-runs\n",
    )
    .expect("initial executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("executable permissions");
    }

    let mut client = client_with_form(temp.path());
    client.send(tool_call(
        90,
        json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
    ));
    let initial_prompt = client.receive();
    assert_eq!(initial_prompt["method"], "elicitation/create");
    client.send(json!({
        "jsonrpc": "2.0",
        "id": initial_prompt["id"],
        "result": { "action": "accept", "content": { "approve": true } }
    }));
    assert_eq!(
        client.receive_with_id(90)["result"]["structuredContent"]["merge_readiness"]["verification"]
            [0]["status"],
        "passed"
    );
    assert_eq!(
        fs::read_to_string(temp.path().join(".local/verification-runs")).expect("initial run"),
        "original"
    );
    fs::remove_file(temp.path().join(".local/verification-runs")).expect("clear run marker");

    client.send(tool_call(
        91,
        json!({ "path": ".", "detail": "full", "verify": ["unit"] }),
    ));
    let prompt = client.receive();
    assert_eq!(prompt["method"], "elicitation/create");
    fs::write(
        &executable,
        "#!/bin/sh\nprintf replacement >> .local/verification-runs\n",
    )
    .expect("replace ignored executable");
    client.send(json!({
        "jsonrpc": "2.0",
        "id": prompt["id"],
        "result": { "action": "accept", "content": { "approve": true } }
    }));
    let outcome = client.receive_with_id(91)["result"]["structuredContent"]["merge_readiness"]["verification"][0].clone();
    assert_eq!(outcome["status"], "skipped");
    assert_eq!(
        outcome["limitations"][0],
        "check definition changed before execution"
    );
    assert!(!temp.path().join(".local/verification-runs").exists());
    assert!(client.close().success());
}
