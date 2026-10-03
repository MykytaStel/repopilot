//! Session state, the Cursor follow-up, and the blocks both agents share.

use crate::support::*;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[test]
fn session_state_stays_out_of_git_status() {
    let temp = repo(&[]);
    let root = temp.path();
    start(root);
    start(root);
    write(
        root,
        "src/cart.test.ts",
        &tests_with_skips(&["applies the discount"]),
    );
    assert_eq!(stop(root).0, Some(2));

    let status = git(root, &["status", "--porcelain"]);
    assert!(!status.contains(".repopilot"), "{status}");
    let exclude = fs::read_to_string(root.join(".git/info/exclude")).expect("exclude");
    assert_eq!(
        exclude.matches("/.repopilot/snapshot.json\n").count(),
        1,
        "{exclude}"
    );
    assert_eq!(
        exclude.matches("/.repopilot/cache/\n").count(),
        1,
        "{exclude}"
    );
}

#[test]
fn cursor_follows_up_once_per_session_and_not_on_review_context() {
    let temp = repo(&[]);
    let root = temp.path();
    assert!(run(root, CURSOR_SNAPSHOT, "{}").status.success());
    let follow_up = |root: &Path| -> Value {
        let output = run(root, CURSOR_GUARD, CURSOR_STOP);
        assert!(output.status.success(), "{output:?}");
        serde_json::from_slice(&output.stdout).expect("cursor hook prints JSON")
    };

    write(
        root,
        "package.json",
        r#"{"dependencies":{"left-pad":"1.3.0"}}"#,
    );
    assert_eq!(follow_up(root), serde_json::json!({}));

    write(
        root,
        "src/cart.test.ts",
        &tests_with_skips(&["applies the discount"]),
    );
    let message = follow_up(root)["followup_message"]
        .as_str()
        .expect("follow-up")
        .to_string();
    assert!(
        message.contains("test skipped — src/cart.test.ts"),
        "{message}"
    );
    assert_eq!(
        follow_up(root),
        serde_json::json!({}),
        "raised once per session"
    );
}

#[test]
fn claude_code_and_cursor_hooks_share_identical_blocks() {
    let blocks = |script: &str| -> Vec<String> {
        let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(script))
            .expect("hook script");
        text.split("# --- shared: ")
            .skip(1)
            .map(|rest| {
                rest.split("# --- end shared ---")
                    .next()
                    .unwrap()
                    .to_string()
            })
            .collect()
    };
    for (claude, cursor) in [
        (CLAUDE_GUARD, CURSOR_GUARD),
        (CLAUDE_SNAPSHOT, CURSOR_SNAPSHOT),
    ] {
        let shared = blocks(claude);
        assert_eq!(shared.len(), 1, "{claude}");
        assert_eq!(shared, blocks(cursor), "{claude} and {cursor} drifted");
    }
}
