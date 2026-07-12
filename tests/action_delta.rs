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
fn review_action_builds_stable_base_head_delta_artifact() {
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
{"review":{"in_diff_findings":1,"tiered_signals":{"definitely":0,"maybe":0,"noise":0,"total":0}},"tiered_signals":{"definitely":[],"maybe":[],"noise":[]},"findings":[]}
JSON
  printf '{"version":"2.1.0","runs":[]}\n' > "$sarif"
  exit 0
fi
if [[ "$(git rev-parse HEAD)" == "$FAKE_BASE_REVISION" ]]; then
  cat > "$output" <<'JSON'
{"findings":[{"id":"changed","occurrence_key":"before","title":"Changed before","risk":{"priority":"P2"},"evidence":[{"path":"./source.rs","line_start":1}]},{"id":"resolved","occurrence_key":"resolved","title":"Resolved","risk":{"priority":"P2"},"evidence":[{"path":"./old.rs","line_start":1}]}]}
JSON
else
  cat > "$output" <<'JSON'
{"findings":[{"id":"changed","occurrence_key":"after","title":"Changed after","risk":{"priority":"P2"},"evidence":[{"path":"./source.rs","line_start":1}]},{"id":"new","occurrence_key":"new","title":"New","risk":{"priority":"P1"},"evidence":[{"path":"./new.rs","line_start":1}]}]}
JSON
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
        .env("FAKE_BASE_REVISION", &base)
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
    assert_eq!(delta["summary"]["new_findings"], 1);
    assert_eq!(delta["summary"]["changed_findings"], 1);
    assert_eq!(delta["summary"]["resolved_findings"], 1);
    assert_eq!(delta["new_findings"][0]["id"], "new");
    assert_eq!(delta["changed_findings"][0]["id"], "changed");
    assert_eq!(delta["resolved_findings"][0]["id"], "resolved");

    let outputs = fs::read_to_string(github_output).expect("read action outputs");
    assert!(outputs.contains("delta_json_file=repopilot-review-delta.json"));
    assert!(outputs.contains("new_findings_count=1"));
    assert!(outputs.contains("changed_findings_count=1"));
    assert!(outputs.contains("resolved_findings_count=1"));
    let summary =
        fs::read_to_string(root.join("repopilot-review-summary.md")).expect("read review summary");
    assert!(summary.contains("**New findings:** 1"));
    assert!(summary.contains("**Changed findings:** 1"));
    assert!(summary.contains("**Resolved findings:** 1"));
}
