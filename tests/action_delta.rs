#![cfg(unix)]

use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn review_action_builds_delta_through_the_baseline_engine() {
    // Proves the Action's shell glue reads the canonical baseline-engine
    // output correctly, not that the engine itself diffs correctly (that is
    // covered by src/baseline/diff/tests.rs). The fake `repopilot` here
    // stands in for `baseline create` on the base revision and
    // `scan --baseline` on the head revision, returning hand-crafted
    // BaselineJsonReport-shaped JSON so this test exercises exactly the jq
    // queries `repopilot-action-review.sh` runs against real output.
    let temp = tempdir().expect("tempdir");
    let root = temp.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
    git(root, &["config", "user.email", "test@repopilot.local"]);
    fs::write(root.join("source.rs"), "pub fn before() {}\n").expect("write base source");
    git(root, &["add", "source.rs"]);
    git(root, &["commit", "-qm", "base"]);
    let base = git(root, &["rev-parse", "HEAD"]);
    fs::write(root.join("source.rs"), "pub fn after() {}\n").expect("write head source");
    git(root, &["commit", "-qam", "head"]);
    let head = git(root, &["rev-parse", "HEAD"]);

    let fake_bin = root.join("fake-bin");
    fs::create_dir(&fake_bin).expect("create fake bin");
    let fake_repopilot = fake_bin.join("repopilot");
    fs::write(
        &fake_repopilot,
        r#"#!/usr/bin/env bash
set -euo pipefail
command="$1"
shift
subcommand=""
if [[ "$command" == "baseline" ]]; then
  subcommand="$1"
  shift
fi
output=""
sarif=""
while (($#)); do
  case "$1" in
    --output) output="$2"; shift 2 ;;
    --sarif-output) sarif="$2"; shift 2 ;;
    *) shift ;;
  esac
done
if [[ "$command" == "review" ]]; then
  cat > "$output" <<'JSON'
{"merge_readiness":{"verdict":"ready"},"review":{"in_diff_findings":1,"tiered_signals":{"definitely":0,"maybe":0,"noise":0,"total":0}},"tiered_signals":{"definitely":[],"maybe":[],"noise":[]},"findings":[]}
JSON
  printf '{"version":"2.1.0","runs":[]}\n' > "$sarif"
  exit 0
fi
if [[ "$command" == "baseline" && "$subcommand" == "create" ]]; then
  # Stands in for the base revision's stored snapshot; its own content is
  # never read by the shell script, only the head scan's report is.
  cat > "$output" <<'JSON'
{"schema_version":1,"tool":"repopilot","created_at":"2024-01-01T00:00:00Z","root":".","findings":[]}
JSON
  exit 0
fi
if [[ "$command" == "scan" ]]; then
  cat > "$output" <<'JSON'
{"baseline":{"path":"base-baseline.json","new_findings":1,"existing_findings":1,"resolved_findings":1},"resolved":[{"key":"rule.gone:old.rs:deadbeef","rule_id":"rule.gone","severity":"medium","path":"old.rs","message":"Old finding"}],"findings":[{"id":"survives-a-line-move","rule_id":"rule.survives","title":"Survives a line move","risk":{"priority":"P2"},"evidence":[{"path":"./source.rs","line_start":1}],"baseline_status":"existing"},{"id":"new","rule_id":"rule.new","title":"New finding","risk":{"priority":"P1"},"evidence":[{"path":"./new.rs","line_start":1}],"baseline_status":"new"}]}
JSON
  exit 0
fi
"#,
    )
    .expect("write fake repopilot");
    let mut permissions = fs::metadata(&fake_repopilot).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_repopilot, permissions).expect("chmod fake repopilot");

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let github_output = root.join("github-output.txt");
    let output = Command::new("bash")
        .arg(manifest.join("scripts/repopilot-action.sh"))
        .current_dir(root)
        .env_remove("GITHUB_EVENT_PATH")
        .env("PATH", format!("{}:{}", fake_bin.display(), env!("PATH")))
        .env("GITHUB_ACTION_PATH", manifest)
        .env("GITHUB_OUTPUT", &github_output)
        .env("INPUT_COMMAND", "review")
        .env("INPUT_FORMAT", "auto")
        .env("INPUT_BASE", &base)
        .env("INPUT_HEAD", &head)
        .output()
        .expect("run action helper");
    assert!(
        output.status.success(),
        "helper failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let delta: Value = serde_json::from_slice(
        &fs::read(root.join("repopilot-review-delta.json")).expect("read delta"),
    )
    .expect("parse delta");
    assert_eq!(delta["baseline"]["new_findings"], 1);
    assert_eq!(delta["baseline"]["resolved_findings"], 1);
    assert_eq!(
        delta["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|finding| finding["baseline_status"] == "new")
            .count(),
        1,
    );
    assert_eq!(delta["resolved"][0]["key"], "rule.gone:old.rs:deadbeef");

    let outputs = fs::read_to_string(github_output).expect("read action outputs");
    assert!(outputs.contains("delta_json_file=repopilot-review-delta.json"));
    assert!(outputs.contains("new_findings_count=1"));
    // No "changed" bucket exists in the baseline model; the output is kept
    // for compatibility and always reports 0.
    assert!(outputs.contains("changed_findings_count=0"));
    assert!(outputs.contains("resolved_findings_count=1"));
    let summary =
        fs::read_to_string(root.join("repopilot-review-summary.md")).expect("read review summary");
    assert!(summary.contains("**New findings:** 1"));
    assert!(summary.contains("**Resolved findings:** 1"));
    assert!(summary.contains("**Merge readiness:** ready"));
    assert!(
        summary.contains("New finding"),
        "the new finding's title should appear in the summary:\n{summary}"
    );
    assert!(
        !summary.contains("Survives a line move"),
        "an existing (unmoved-key) finding must not be listed as new:\n{summary}"
    );
}
