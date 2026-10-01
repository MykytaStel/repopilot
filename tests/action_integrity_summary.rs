//! The GitHub Action's PR summary lists the checks a change weakened before
//! the proof details, and does not repeat them in the general signal list.
#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

const BEFORE: &str = r#"describe("cart", () => {
  it("sums line items", () => {
    expect(total([2, 3])).toBe(5);
    expect(total([])).toBe(0);
  });
  it("applies the discount", () => {
    expect(total([10], 0.1)).toBe(9);
  });
});
"#;

fn run(root: &Path, program: &str, args: &[&str]) {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .expect("command runs");
    assert!(
        output.status.success(),
        "{program} {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pr_summary_lists_weakened_checks_first() {
    let temp = tempdir().expect("temp repo");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(root.join("src/cart.test.ts"), BEFORE).expect("test file");
    fs::write(root.join("src/session.ts"), "export const ttl = 60;\n").expect("source");
    run(root, "git", &["init", "-q"]);
    run(
        root,
        "git",
        &["config", "user.email", "test@example.invalid"],
    );
    run(root, "git", &["config", "user.name", "Test"]);
    run(root, "git", &["add", "."]);
    run(root, "git", &["commit", "-qm", "before"]);

    let weakened = BEFORE
        .replace("it(\"applies", "it.skip(\"applies")
        .replace("    expect(total([])).toBe(0);\n", "");
    fs::write(root.join("src/cart.test.ts"), weakened).expect("weaken");
    fs::create_dir_all(root.join("src/auth")).expect("auth dir");
    fs::write(
        root.join("src/auth/guard.ts"),
        "export const allow = true;\n",
    )
    .expect("auth");

    run(
        root,
        env!("CARGO_BIN_EXE_repopilot"),
        &["review", ".", "--format", "json", "--output", "review.json"],
    );
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action-review.sh");
    run(
        root,
        "bash",
        &[
            "-c",
            &format!(
                "source \"{}\" && write_review_summary review.json",
                script.display()
            ),
        ],
    );
    let summary = fs::read_to_string(root.join("repopilot-review-summary.md")).expect("summary");

    let weakened_at = summary
        .find("### Checks this change weakened")
        .expect("weakened section");
    let proof_at = summary.find("**Change proof:**").expect("proof line");
    assert!(weakened_at < proof_at, "{summary}");
    for line in [
        "- **test skipped** — `src/cart.test.ts:",
        "- **assertions removed** — `src/cart.test.ts:",
    ] {
        assert_eq!(summary.matches(line).count(), 1, "{line}\n{summary}");
    }
    // Other review signals keep their place in the general list.
    let boundary = summary
        .find("`src/auth/guard.ts`")
        .expect("boundary signal listed");
    assert!(boundary > proof_at, "{summary}");
}

#[test]
fn pr_summary_has_no_weakened_section_without_integrity_signals() {
    let temp = tempdir().expect("temp repo");
    let root = temp.path();
    fs::write(root.join("lib.ts"), "export const value = 1;\n").expect("source");
    run(root, "git", &["init", "-q"]);
    run(
        root,
        "git",
        &["config", "user.email", "test@example.invalid"],
    );
    run(root, "git", &["config", "user.name", "Test"]);
    run(root, "git", &["add", "."]);
    run(root, "git", &["commit", "-qm", "before"]);
    fs::write(root.join("lib.ts"), "export const value = 2;\n").expect("change");

    run(
        root,
        env!("CARGO_BIN_EXE_repopilot"),
        &["review", ".", "--format", "json", "--output", "review.json"],
    );
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action-review.sh");
    run(
        root,
        "bash",
        &[
            "-c",
            &format!(
                "source \"{}\" && write_review_summary review.json",
                script.display()
            ),
        ],
    );
    let summary = fs::read_to_string(root.join("repopilot-review-summary.md")).expect("summary");
    assert!(
        !summary.contains("Checks this change weakened"),
        "{summary}"
    );
    assert!(
        summary.starts_with("## RepoPilot Review\n\n- **"),
        "{summary}"
    );
}
