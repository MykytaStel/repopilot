//! End-to-end coverage for the unified, confidence-tiered review signals
//! (`src/review/signals/tiered.rs`). Unit-level tiering rules live next to the
//! detector; these tests prove the three tiers flow through the real `review`
//! command into the console, Markdown, and JSON outputs (the agent-facing
//! `repopilot_review_change` shape). The boundary-signal proofs in
//! `tests/review_boundary_signals.rs` stay green alongside this: `tiered_signals`
//! is additive and `boundary_signals` still ships unchanged.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

/// An access-control boundary file (definitely tier) plus a plain file that adds
/// a `fetch()` network call (maybe tier).
fn write_definitely_and_maybe(root: &Path) {
    init_repo(root);
    write(root, "src/lib.rs", "pub fn live() {}\n");
    commit_all(root, "initial");

    write(root, "src/auth/session.ts", "export const session = {};\n");
    write(
        root,
        "src/service/loader.ts",
        "export async function load() {\n  const res = await fetch(\"https://example.com/data\");\n  return res.json();\n}\n",
    );
}

#[test]
fn review_groups_signals_into_tiers_in_json() {
    let temp = tempdir().expect("failed to create temp dir");
    write_definitely_and_maybe(temp.path());

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);

    let tiered = &json["tiered_signals"];
    let definitely = tiered["definitely"].as_array().expect("definitely array");
    let maybe = tiered["maybe"].as_array().expect("maybe array");
    let noise = tiered["noise"].as_array().expect("noise array");

    // Definitely: the boundary signal, escalated from the access-control change.
    assert!(definitely.iter().any(|signal| {
        signal["family"] == "boundary"
            && signal["tier"] == "definitely-sensitive"
            && signal["path"] == "src/auth/session.ts"
            && signal["kind"] == "boundary.access-control"
            && signal["confidence"] == "HIGH"
            && signal["provenance"]["analysis_scope"] == "git-diff"
            && signal["signal_id"]
                .as_str()
                .is_some_and(|id| id.len() == 16)
    }));

    // Maybe: the behavioral network call added in an ordinary file.
    assert!(maybe.iter().any(|signal| {
        signal["family"] == "behavioral"
            && signal["tier"] == "maybe-sensitive"
            && signal["headline"] == "network call added"
            && signal["path"] == "src/service/loader.ts"
    }));

    // No noise note when other tiers already point the eye.
    assert!(noise.is_empty());

    // The `review` summary counts mirror the arrays exactly.
    let counts = &json["review"]["tiered_signals"];
    assert_eq!(counts["definitely"], definitely.len() as u64);
    assert_eq!(counts["maybe"], maybe.len() as u64);
    assert_eq!(counts["noise"], noise.len() as u64);
    assert_eq!(
        counts["total"],
        (definitely.len() + maybe.len() + noise.len()) as u64
    );
    for field in [
        "diff_loading_us",
        "review_signals_us",
        "gating_us",
        "rendering_us",
    ] {
        assert!(
            json["review_timings"][field].is_number(),
            "missing review timing {field}"
        );
    }

    // Backward-thoughtful: the legacy boundary view still ships unchanged.
    assert!(
        !json["boundary_signals"]
            .as_array()
            .expect("boundary_signals array")
            .is_empty()
    );
    assert_eq!(json["review"]["boundary_signals"], 1);
}

#[test]
fn review_gate_is_opt_in_and_blocks_definitely_sensitive_signals() {
    let temp = tempdir().expect("failed to create temp dir");
    write_definitely_and_maybe(temp.path());

    let advisory = repopilot()
        .args(["review", "."])
        .current_dir(temp.path())
        .output()
        .expect("run advisory review");
    assert!(advisory.status.success());

    let blocking = repopilot()
        .args(["review", ".", "--fail-on-review", "definitely"])
        .current_dir(temp.path())
        .output()
        .expect("run blocking review");
    assert_eq!(blocking.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&blocking.stderr).contains("review gate failed"),
        "{}",
        String::from_utf8_lossy(&blocking.stderr)
    );
}

#[test]
fn review_feedback_suppresses_signal_by_kind_and_path_glob() {
    let temp = tempdir().expect("failed to create temp dir");
    write_definitely_and_maybe(temp.path());
    write(
        temp.path(),
        ".repopilot/feedback.yml",
        r#"
suppressions:
  - kind: boundary.access-control
    path: "src/auth/**"
    reason: reviewed boundary migration
    expires: "2099-01-01"
"#,
    );

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);
    let signal = json["tiered_signals"]["definitely"]
        .as_array()
        .unwrap()
        .iter()
        .find(|signal| signal["kind"] == "boundary.access-control")
        .expect("boundary signal");
    assert_eq!(signal["suppressed"], true);
    assert_eq!(signal["gate_eligible"], false);
    assert_eq!(signal["suppression_reason"], "reviewed boundary migration");
    assert_eq!(json["local_feedback"]["suppressed_review_signals_count"], 1);

    let gated = repopilot()
        .args(["review", ".", "--fail-on-review", "definitely"])
        .current_dir(temp.path())
        .output()
        .expect("run gated review");
    assert!(gated.status.success());
}

#[test]
fn review_surfaces_taint_flow_in_json() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write(
        temp.path(),
        "src/handler.ts",
        "export function findUser() { return null; }\n",
    );
    commit_all(temp.path(), "initial");

    write(
        temp.path(),
        "src/handler.ts",
        r#"export function findUser(req: Request) {
  const id = req.query.id;
  return db.query("SELECT * FROM users WHERE id = " + id);
}
"#,
    );

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);
    let definitely = json["tiered_signals"]["definitely"]
        .as_array()
        .expect("definitely array");

    assert!(definitely.iter().any(|signal| {
        signal["family"] == "taint"
            && signal["headline"] == "untrusted input reaches raw SQL"
            && signal["path"] == "src/handler.ts"
            && signal["line"] == 3
    }));
}

#[test]
fn review_respects_disabled_taint_config() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write(temp.path(), "repopilot.toml", "[taint]\nenabled = false\n");
    write(
        temp.path(),
        "src/handler.ts",
        "export function findUser() { return null; }\n",
    );
    commit_all(temp.path(), "initial");

    write(
        temp.path(),
        "src/handler.ts",
        r#"export function findUser(req: Request) {
  const id = req.query.id;
  return db.query("SELECT * FROM users WHERE id = " + id);
}
"#,
    );

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);
    for tier in ["definitely", "maybe", "noise"] {
        assert!(
            json["tiered_signals"][tier]
                .as_array()
                .expect("tier array")
                .iter()
                .all(|signal| signal["family"] != "taint")
        );
    }
}

#[test]
fn review_surfaces_large_diff_as_noise_tier() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write(temp.path(), "README.md", "# demo\n");
    commit_all(temp.path(), "initial");

    // Six plain files, ~40 lines each: a big diff with no boundary / behavioral /
    // algorithmic / taint hit (`.txt` is not one of the analyzed grammars).
    let body = "lorem ipsum dolor sit amet\n".repeat(40);
    for index in 0..6 {
        write(temp.path(), &format!("notes/note_{index}.txt"), &body);
    }

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);

    let tiered = &json["tiered_signals"];
    assert!(tiered["definitely"].as_array().unwrap().is_empty());
    assert!(tiered["maybe"].as_array().unwrap().is_empty());

    let noise = tiered["noise"].as_array().expect("noise array");
    assert!(
        noise.iter().any(|signal| {
            signal["family"] == "volume" && signal["tier"] == "large-diff-or-noise"
        })
    );
    assert_eq!(
        json["review"]["tiered_signals"]["noise"],
        noise.len() as u64
    );
}

#[test]
fn review_console_shows_tier_groups() {
    let temp = tempdir().expect("failed to create temp dir");
    write_definitely_and_maybe(temp.path());

    let output = repopilot()
        .args(["review", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run review");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Review signals"));
    assert!(stdout.contains("Definitely sensitive"));
    assert!(stdout.contains("Maybe sensitive"));
    assert!(stdout.contains("access control"));
    assert!(stdout.contains("network call added"));
    assert!(stdout.contains("src/service/loader.ts"));
}

#[test]
fn review_markdown_shows_tier_groups() {
    let temp = tempdir().expect("failed to create temp dir");
    write_definitely_and_maybe(temp.path());

    let output = repopilot()
        .args(["review", ".", "--format", "markdown"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run review");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("## Review Signals (preview)"));
    assert!(stdout.contains("### Definitely sensitive"));
    assert!(stdout.contains("### Maybe sensitive"));
    assert!(stdout.contains("network call added"));
}

fn run_review_json(root: &Path, args: &[&str]) -> Value {
    let output = repopilot()
        .args(args)
        .current_dir(root)
        .output()
        .expect("failed to run review");

    assert!(output.status.success());
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
