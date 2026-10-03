//! Without configured verification, a change with nothing flagged is
//! `PASS (not verified)`; evidence still keeps `REVIEW`, and configuring a
//! check restores the plain Change Proof mapping.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

const TESTS: &str = "import { expect, it } from \"vitest\";\n\nit(\"adds\", () => {\n  expect(1 + 2).toBe(3);\n});\n";
const CHECK: &str = "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"npm\"\nargs = [\"test\"]\npaths = [\"src/**\", \"README.md\"]\n";

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

fn repo(root: &Path, extra: &[(&str, &str)]) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "repopilot@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(root.join("README.md"), "# Cart\n").expect("readme");
    fs::write(root.join("src/cart.test.ts"), TESTS).expect("tests");
    for (path, content) in extra {
        fs::write(root.join(path), content).expect("extra file");
    }
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "before"]);
}

fn review(root: &Path, format: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["review", ".", "--format", format])
        .current_dir(root)
        .env("NO_COLOR", "1")
        .output()
        .expect("review runs");
    assert!(output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn json(root: &Path) -> Value {
    serde_json::from_str(&review(root, "json")).expect("review JSON")
}

#[test]
fn a_change_with_nothing_flagged_passes_unverified() {
    let temp = tempdir().expect("temp repo");
    let root = temp.path();
    repo(root, &[]);
    fs::write(root.join("README.md"), "# Cart\n\nSums line items.\n").expect("edit");

    let console = review(root, "console");
    assert!(
        console.contains("Decision: PASS (not verified)\n"),
        "{console}"
    );
    assert!(!console.contains("Reasons:"), "{console}");
    assert!(!console.contains("Proof policy:"), "{console}");
    assert!(
        console.contains("Verification: not configured"),
        "{console}"
    );

    let report = json(root);
    assert_eq!(
        report["decision"]["verdict"], "PASS",
        "{}",
        report["decision"]
    );
    assert_eq!(report["change_proof"]["verdict"], "REVIEW");
    let limitations = report["decision"]["limitations"].to_string();
    assert!(limitations.contains("not verified"), "{limitations}");

    let markdown = review(root, "markdown");
    assert!(markdown.contains("`PASS` (not verified)"), "{markdown}");
}

#[test]
fn a_review_signal_keeps_review_and_lists_the_evidence_first() {
    let temp = tempdir().expect("temp repo");
    let root = temp.path();
    repo(root, &[]);
    fs::write(
        root.join("src/cart.test.ts"),
        TESTS.replace("it(\"adds\"", "it.skip(\"adds\""),
    )
    .expect("skip");

    let console = review(root, "console");
    assert!(
        console.contains("Decision: REVIEW (Change Proof: REVIEW)"),
        "{console}"
    );
    let reasons: Vec<&str> = console
        .split("Reasons:\n")
        .nth(1)
        .expect("reasons")
        .lines()
        .take_while(|line| line.starts_with("  - "))
        .collect();
    assert!(
        reasons.first().is_some_and(|line| line.contains("signal")),
        "{reasons:?}"
    );
    assert_eq!(
        reasons.last().copied(),
        Some("  - Not verified: no verification checks are configured."),
        "{reasons:?}"
    );
    assert!(!console.contains("No sufficient proof policy"), "{console}");
    assert_eq!(json(root)["decision"]["verdict"], "REVIEW");
}

#[test]
fn a_configured_check_keeps_the_change_proof_mapping() {
    let temp = tempdir().expect("temp repo");
    let root = temp.path();
    repo(root, &[("repopilot.toml", CHECK)]);
    fs::write(root.join("README.md"), "# Cart\n\nSums line items.\n").expect("edit");

    let console = review(root, "console");
    assert!(
        console.contains("Decision: REVIEW (Change Proof: REVIEW)"),
        "{console}"
    );
    assert!(console.contains("Proof policy:"), "{console}");
    assert_eq!(json(root)["decision"]["verdict"], "REVIEW");
}
