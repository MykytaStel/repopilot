//! Which signals stop the agent, and how often.

use crate::support::*;

#[test]
fn review_context_changes_do_not_stop_the_agent() {
    let temp = repo(&[]);
    let root = temp.path();
    start(root);
    write(
        root,
        "package.json",
        r#"{"dependencies":{"left-pad":"1.3.0"}}"#,
    );
    write(
        root,
        ".github/workflows/ci.yml",
        &WORKFLOW.replace(
            "    steps:\n",
            "    steps:\n      - uses: actions/setup-node@v4\n",
        ),
    );

    let (code, feedback) = stop(root);
    assert_eq!(
        code,
        Some(0),
        "a dependency bump is not a weakened check: {feedback}"
    );
}

#[test]
fn an_answered_signal_stops_the_agent_once_per_session() {
    let temp = repo(&[]);
    let root = temp.path();
    start(root);
    write(
        root,
        "src/cart.test.ts",
        &tests_with_skips(&["applies the discount"]),
    );

    let (code, feedback) = stop(root);
    assert_eq!(code, Some(2), "{feedback}");
    assert!(
        feedback.contains("test skipped — src/cart.test.ts"),
        "{feedback}"
    );
    let (code, feedback) = stop(root);
    assert_eq!(
        code,
        Some(0),
        "an explained skip must not stop the next turn: {feedback}"
    );

    let both = tests_with_skips(&["applies the discount", "sums line items"]);
    write(root, "src/cart.test.ts", &both);
    let (code, feedback) = stop(root);
    assert_eq!(code, Some(2), "a new skip is raised: {feedback}");
    assert!(feedback.contains("\"sums line items\""), "{feedback}");
    assert!(!feedback.contains("\"applies the discount\""), "{feedback}");

    write(root, "src/cart.test.ts", TESTS);
    assert_eq!(stop(root).0, Some(0));
    write(
        root,
        "src/cart.test.ts",
        &tests_with_skips(&["applies the discount"]),
    );
    assert_eq!(
        stop(root).0,
        Some(2),
        "a skip that returns after a restore is raised again"
    );

    write(root, "src/cart.test.ts", TESTS);
    start(root);
    write(
        root,
        "src/cart.test.ts",
        &tests_with_skips(&["applies the discount"]),
    );
    assert_eq!(
        stop(root).0,
        Some(2),
        "a new session starts with nothing raised"
    );
}

#[test]
fn a_skip_behind_many_sensitive_files_still_stops_the_agent() {
    let workflows: Vec<(String, String)> = (0..24)
        .map(|index| {
            (
                format!(".github/workflows/w{index}.yml"),
                WORKFLOW.to_string(),
            )
        })
        .collect();
    let extra: Vec<(&str, &str)> = workflows
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect();
    let temp = repo(&extra);
    let root = temp.path();
    start(root);
    for (path, content) in &workflows {
        write(
            root,
            path,
            &content.replace("ubuntu-latest", "ubuntu-24.04"),
        );
    }
    assert_eq!(
        stop(root).0,
        Some(0),
        "workflow edits alone are review context"
    );

    write(
        root,
        "src/cart.test.ts",
        &tests_with_skips(&["applies the discount"]),
    );
    let (code, feedback) = stop(root);
    assert_eq!(code, Some(2), "{feedback}");
    assert!(
        feedback.contains("test skipped — src/cart.test.ts"),
        "{feedback}"
    );
}

#[test]
fn a_configured_review_gate_does_not_disable_the_stop_hook() {
    let temp = repo(&[("repopilot.toml", "[review]\nfail_on = \"definitely\"\n")]);
    let root = temp.path();
    start(root);
    write(
        root,
        "src/cart.test.ts",
        &TESTS.replace("it(\"sums", "it.only(\"sums"),
    );

    let (code, feedback) = stop(root);
    assert_eq!(code, Some(2), "{feedback}");
    assert!(feedback.contains("focused test committed"), "{feedback}");
}
