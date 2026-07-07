//! End-to-end proof that `repopilot review` runs boundary, behavioral,
//! algorithmic, and taint-lite detection in one unified pass
//! (`src/review/signal_pass.rs`): a single review invocation over one changed
//! file that touches all four surfaces produces all four delta types at once.
//! This is the black-box counterpart to the pipeline refactor — it doesn't
//! (and can't, from the CLI) observe parse counts, but it pins that the
//! unification changed nothing about *what* gets detected.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn one_review_pass_surfaces_all_four_delta_types_together() {
    let temp = tempdir().expect("failed to create temp dir");
    let root = temp.path();
    init_repo(root);

    // `src/auth/...` is a boundary access-control path. The pre-change body has
    // a plain single loop; the post-change body adds a nested loop
    // (algorithmic), a network call (behavioral), and a SQL-injection taint
    // flow — all in one changed file, all within the diff's changed ranges.
    write(
        root,
        "src/auth/session.ts",
        "export function process(items: number[]) {\n  for (const i of items) {\n    console.log(i);\n  }\n}\n",
    );
    commit_all(root, "initial");

    write(
        root,
        "src/auth/session.ts",
        r#"export function process(items: number[]) {
  for (const i of items) {
    for (const j of items) {
      console.log(i + j);
    }
  }
}

export async function load() {
  const res = await fetch("https://example.com/data");
  return res.json();
}

export function lookup(req: any) {
  const id = req.query.id;
  db.query("SELECT * FROM users WHERE id = " + id);
}
"#,
    );

    let json = run_review_json(root, &["review", ".", "--format", "json"]);

    let boundary = json["boundary_signals"]
        .as_array()
        .expect("boundary_signals array");
    assert!(
        boundary
            .iter()
            .any(|signal| signal["path"] == "src/auth/session.ts"
                && signal["category"] == "access-control"),
        "expected a boundary signal: {boundary:#?}"
    );

    let tiered = &json["tiered_signals"];
    let all_tiers = tiered["definitely"]
        .as_array()
        .expect("definitely array")
        .iter()
        .chain(tiered["maybe"].as_array().expect("maybe array"))
        .chain(tiered["noise"].as_array().expect("noise array"))
        .collect::<Vec<_>>();

    assert!(
        all_tiers
            .iter()
            .any(|signal| signal["family"] == "algorithmic"
                && signal["path"] == "src/auth/session.ts"),
        "expected an algorithmic signal: {all_tiers:#?}"
    );
    assert!(
        all_tiers
            .iter()
            .any(|signal| signal["family"] == "behavioral"
                && signal["headline"] == "network call added"
                && signal["path"] == "src/auth/session.ts"),
        "expected a behavioral signal: {all_tiers:#?}"
    );
    assert!(
        all_tiers
            .iter()
            .any(|signal| signal["family"] == "taint" && signal["path"] == "src/auth/session.ts"),
        "expected a taint signal: {all_tiers:#?}"
    );
}

fn run_review_json(root: &Path, args: &[&str]) -> Value {
    let output = repopilot()
        .args(args)
        .current_dir(root)
        .output()
        .expect("failed to run review");

    assert!(
        output.status.success(),
        "review failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("expected JSON output")
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create parent dir");
    }
    fs::write(path, content).expect("failed to write file");
}

fn init_repo(root: &Path) {
    git(root, &["init"]);
    git(root, &["config", "user.email", "repopilot@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(root, &["commit", "-m", message]);
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("failed to run git");

    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}
