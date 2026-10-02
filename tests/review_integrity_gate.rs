//! Pins the 0.24 behavior change: `review --fail-on-review definitely` fails
//! on a `definitely` integrity signal (a committed focused test), while a
//! `maybe` integrity signal such as a skipped test is reported without
//! failing the gate.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::tempdir;

const CART_TEST: &str = r#"import { total } from "./cart";

describe("cart", () => {
  it("sums line items", () => {
    expect(total([2, 3])).toBe(5);
  });
  it("applies the discount", () => {
    expect(total([10], 0.1)).toBe(9);
  });
});
"#;

#[test]
fn a_committed_focused_test_fails_the_definitely_gate() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    seed(root);
    let focused = CART_TEST.replace("it(\"applies", "it.only(\"applies");
    fs::write(root.join("src/cart.test.ts"), focused).expect("focus test");

    let gated = review(root, &["--fail-on-review", "definitely"]);
    assert_eq!(gated.status.code(), Some(1), "{}", stderr(&gated));
    assert!(
        stderr(&gated).contains("review gate failed"),
        "{}",
        stderr(&gated)
    );

    let json = review(root, &["--format", "json"]);
    assert!(stdout(&json).contains("integrity.test-focused"));
}

#[test]
fn a_skipped_test_is_reported_without_failing_the_definitely_gate() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    seed(root);
    let skipped = CART_TEST.replace("it(\"applies", "it.skip(\"applies");
    fs::write(root.join("src/cart.test.ts"), skipped).expect("skip test");

    let gated = review(root, &["--fail-on-review", "definitely"]);
    assert!(gated.status.success(), "{}", stderr(&gated));

    let json = review(root, &["--format", "json"]);
    assert!(stdout(&json).contains("integrity.test-skipped"));
}

#[test]
fn console_lists_weakened_checks_right_after_the_decision() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    seed(root);
    let skipped = CART_TEST.replace("it(\"applies", "it.skip(\"applies");
    fs::write(root.join("src/cart.test.ts"), skipped).expect("skip test");

    let console = stdout(&review(root, &[]));
    let decision = console.find("Decision:").expect("decision line");
    let weakened = console
        .find("Checks this change weakened")
        .expect("weakened checks block");
    let evidence = console.find("Evidence scope:").expect("evidence line");
    let signals = console
        .find("Review signals [preview]")
        .expect("review signals section");
    assert!(
        decision < weakened && weakened < evidence && evidence < signals,
        "{console}"
    );
    assert!(
        console.contains("  \u{2691} test skipped \u{2014} src/cart.test.ts:"),
        "{console}"
    );
}

#[test]
fn console_has_no_weakened_block_without_integrity_signals() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    seed(root);
    fs::write(
        root.join("src/cart.ts"),
        "export function total(items: number[]): number {\n  return items.length;\n}\n",
    )
    .expect("edit source");
    let console = stdout(&review(root, &[]));
    assert!(
        !console.contains("Checks this change weakened"),
        "{console}"
    );
}

fn seed(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("src dir");
    fs::write(
        root.join("src/cart.ts"),
        "export function total(items: number[], discount = 0): number {\n  return items.reduce((a, b) => a + b, 0) * (1 - discount);\n}\n",
    )
    .expect("write source");
    fs::write(root.join("src/cart.test.ts"), CART_TEST).expect("write test");
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "repopilot@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "initial"]);
}

fn review(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .arg("review")
        .arg(".")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run review")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?}");
}
