use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
#[cfg(unix)]
use std::process::Command;
#[cfg(unix)]
use tempfile::tempdir;

#[test]
fn action_exposes_typed_receipt_input_and_output() {
    let action_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("action.yml");
    let action = fs::read_to_string(action_path).expect("read action.yml");

    assert!(action.contains("receipt:"));
    assert!(action.contains("receipt-file:"));
    assert!(action.contains("INPUT_RECEIPT: ${{ inputs.receipt }}"));
    assert!(action.contains("scripts/repopilot-action.sh"));

    let helper_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action.sh");
    let helper = fs::read_to_string(helper_path).expect("read action helper");
    assert!(helper.contains("Receipt output is only supported by 'scan'"));
    assert!(helper.contains("RUN_ARGS+=(--receipt \"$RECEIPT\")"));
    assert!(helper.contains("receipt_file=$RECEIPT"));
}

#[test]
fn action_exposes_priority_and_rule_parity_inputs() {
    let action_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("action.yml");
    let action = fs::read_to_string(action_path).expect("read action.yml");

    for input in ["fail-on-priority:", "min-priority:", "rule:", "timing:"] {
        assert!(action.contains(input), "missing action input {input}");
    }

    assert!(action.contains("INPUT_FAIL_ON_PRIORITY: ${{ inputs.fail-on-priority }}"));
    assert!(action.contains("INPUT_MIN_PRIORITY: ${{ inputs.min-priority }}"));
    assert!(action.contains("INPUT_RULE: ${{ inputs.rule }}"));
    assert!(action.contains("INPUT_TIMING: ${{ inputs.timing }}"));

    let helper_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action.sh");
    let helper = fs::read_to_string(helper_path).expect("read action helper");
    assert!(helper.contains("RUN_ARGS+=(--fail-on-priority \"$FAIL_ON_PRIORITY\")"));
    assert!(helper.contains("RUN_ARGS+=(--min-priority \"$MIN_PRIORITY\")"));
    assert!(helper.contains("RUN_ARGS+=(--rule \"$RULE\")"));
    assert!(helper.contains("RUN_ARGS+=(--timing)"));
}

#[test]
fn action_command_surface_is_scan_review_ai_context() {
    let action_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("action.yml");
    let action = fs::read_to_string(action_path).expect("read action.yml");

    assert!(action.contains("scan | review | ai-context"));
    assert!(action.contains("JSON for review"));
    // compare and doctor were removed from the product surface.
    assert!(!action.contains("doctor"));
    assert!(!action.contains("compare"));

    let helper_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action.sh");
    let helper = fs::read_to_string(helper_path).expect("read action helper");
    assert!(helper.contains("ai-context) RUN_ARGS=(ai context \"$PATH_INPUT\") ;;"));
    assert!(!helper.contains("doctor)"));
    assert!(helper.contains("SARIF output and upload are only supported by 'scan'"));
    assert!(helper.contains("GITHUB_STEP_SUMMARY"));
    assert!(!helper.contains("ai-context|vibe"));
}

#[test]
fn action_exposes_review_outputs_and_fork_safe_defaults() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let action = fs::read_to_string(root.join("action.yml")).expect("read action.yml");
    let workflow = fs::read_to_string(root.join(".github/workflows/repopilot-pr-review.yml"))
        .expect("read reusable workflow");

    for output in [
        "review-json-file:",
        "review-sarif-file:",
        "delta-json-file:",
        "conclusion:",
        "findings-count:",
        "new-findings-count:",
        "changed-findings-count:",
        "resolved-findings-count:",
        "signals-count:",
        "gate-result:",
    ] {
        assert!(action.contains(output), "missing action output {output}");
    }
    assert!(action.contains("scripts/install-action-binary.sh"));
    assert!(action.contains("default: \"false\""));
    assert!(workflow.contains("fetch-depth: 0"));
    assert!(workflow.contains("github.event.pull_request.head.sha || github.sha"));
    assert!(workflow.contains("repopilot-review-delta.json"));
    assert!(
        workflow.contains("actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7")
    );
    assert!(!workflow.contains("actions/upload-artifact@v7"));
    assert!(!workflow.contains("pull_request_target"));
}

#[test]
#[cfg(unix)]
fn action_helper_routes_auto_scan_to_sarif_output() {
    let temp = tempdir().expect("tempdir");
    let capture = temp.path().join("args.txt");
    let github_output = temp.path().join("github-output.txt");
    let fake_bin = temp.path().join("bin");
    fs::create_dir(&fake_bin).expect("create fake bin");
    let fake_repopilot = fake_bin.join("repopilot");
    fs::write(
        &fake_repopilot,
        r#"#!/usr/bin/env bash
printf '%s\n' "$@" > "$CAPTURE_ARGS"
"#,
    )
    .expect("write fake repopilot");
    let mut permissions = fs::metadata(&fake_repopilot).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_repopilot, permissions).expect("chmod fake repopilot");

    let helper = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action.sh");
    let output = Command::new("bash")
        .arg(helper)
        .env_remove("GITHUB_EVENT_PATH")
        .env("PATH", format!("{}:{}", fake_bin.display(), env!("PATH")))
        .env("CAPTURE_ARGS", &capture)
        .env("GITHUB_OUTPUT", &github_output)
        .env("INPUT_COMMAND", "scan")
        .env("INPUT_FORMAT", "auto")
        .env("INPUT_PATH", "src")
        .output()
        .expect("run helper");

    assert!(
        output.status.success(),
        "helper failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let args = fs::read_to_string(capture).expect("read captured args");
    assert_eq!(
        args.lines().collect::<Vec<_>>(),
        vec![
            "scan",
            "src",
            "--format",
            "sarif",
            "--output",
            "repopilot-results.sarif"
        ]
    );
    assert!(
        fs::read_to_string(github_output)
            .expect("read github output")
            .contains("sarif_file=repopilot-results.sarif")
    );
}

#[test]
#[cfg(unix)]
fn action_helper_rejects_receipt_for_non_scan_before_running_repopilot() {
    let temp = tempdir().expect("tempdir");
    let capture = temp.path().join("args.txt");
    let fake_bin = temp.path().join("bin");
    fs::create_dir(&fake_bin).expect("create fake bin");
    let fake_repopilot = fake_bin.join("repopilot");
    fs::write(
        &fake_repopilot,
        r#"#!/usr/bin/env bash
printf '%s\n' "$@" > "$CAPTURE_ARGS"
"#,
    )
    .expect("write fake repopilot");
    let mut permissions = fs::metadata(&fake_repopilot).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_repopilot, permissions).expect("chmod fake repopilot");

    let helper = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action.sh");
    let output = Command::new("bash")
        .arg(helper)
        .env_remove("GITHUB_EVENT_PATH")
        .env("PATH", format!("{}:{}", fake_bin.display(), env!("PATH")))
        .env("CAPTURE_ARGS", &capture)
        .env("INPUT_COMMAND", "review")
        .env("INPUT_RECEIPT", "receipt.json")
        .output()
        .expect("run helper");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("Receipt output is only supported by 'scan'")
    );
    assert!(
        !capture.exists(),
        "validation should fail before invoking repopilot"
    );
}

#[test]
#[cfg(unix)]
fn action_helper_writes_markdown_job_summary() {
    let temp = tempdir().expect("tempdir");
    let capture = temp.path().join("args.txt");
    let step_summary = temp.path().join("summary.md");
    let fake_bin = temp.path().join("bin");
    fs::create_dir(&fake_bin).expect("create fake bin");
    let fake_repopilot = fake_bin.join("repopilot");
    fs::write(
        &fake_repopilot,
        r#"#!/usr/bin/env bash
printf '%s\n' "$@" > "$CAPTURE_ARGS"
printf '# Fake RepoPilot Report\n\n- ok\n'
"#,
    )
    .expect("write fake repopilot");
    let mut permissions = fs::metadata(&fake_repopilot).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_repopilot, permissions).expect("chmod fake repopilot");

    let helper = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action.sh");
    let output = Command::new("bash")
        .arg(helper)
        .env_remove("GITHUB_EVENT_PATH")
        .env("PATH", format!("{}:{}", fake_bin.display(), env!("PATH")))
        .env("CAPTURE_ARGS", &capture)
        .env("GITHUB_STEP_SUMMARY", &step_summary)
        .env("INPUT_COMMAND", "ai-context")
        .env("INPUT_FORMAT", "auto")
        .env("INPUT_PATH", ".")
        .output()
        .expect("run helper");

    assert!(
        output.status.success(),
        "helper failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Fake RepoPilot Report"));

    let summary = fs::read_to_string(step_summary).expect("read step summary");
    assert!(summary.contains("## RepoPilot"));
    assert!(summary.contains("Exit status:** passed"));
    assert!(summary.contains("# Fake RepoPilot Report"));
}

#[test]
#[cfg(unix)]
fn action_helper_emits_review_artifacts_outputs_and_deferred_failure() {
    let temp = tempdir().expect("tempdir");
    let capture = temp.path().join("args.txt");
    let github_output = temp.path().join("github-output.txt");
    let fake_bin = temp.path().join("bin");
    fs::create_dir(&fake_bin).expect("create fake bin");
    let fake_repopilot = fake_bin.join("repopilot");
    fs::write(
        &fake_repopilot,
        r#"#!/usr/bin/env bash
printf '%s\n' "$@" > "$CAPTURE_ARGS"
output=""
sarif=""
while (($#)); do
  case "$1" in
    --output) output="$2"; shift 2 ;;
    --sarif-output) sarif="$2"; shift 2 ;;
    *) shift ;;
  esac
done
cat > "$output" <<'JSON'
{
  "review": {
    "in_diff_findings": 1,
    "tiered_signals": { "definitely": 1, "maybe": 0, "noise": 0, "total": 1 }
  },
  "review_gate": { "status": "failed" },
  "tiered_signals": {
    "definitely": [{
      "headline": "Access control changed",
      "detail": "Role check changed",
      "path": "src/auth.rs",
      "line_start": 4,
      "suppressed": false
    }],
    "maybe": [],
    "noise": []
  },
  "findings": [{
    "title": "Finding",
    "in_diff": true,
    "evidence": [{ "path": "src/auth.rs", "line_start": 4 }]
  }]
}
JSON
printf '{"version":"2.1.0","runs":[]}\n' > "$sarif"
exit 1
"#,
    )
    .expect("write fake repopilot");
    let mut permissions = fs::metadata(&fake_repopilot).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_repopilot, permissions).expect("chmod fake repopilot");

    let helper = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action.sh");
    let output = Command::new("bash")
        .arg(helper)
        .env_remove("GITHUB_EVENT_PATH")
        .current_dir(temp.path())
        .env("PATH", format!("{}:{}", fake_bin.display(), env!("PATH")))
        .env("CAPTURE_ARGS", &capture)
        .env("GITHUB_OUTPUT", &github_output)
        .env("REPOPILOT_DEFER_FAILURE", "true")
        .env("INPUT_COMMAND", "review")
        .env("INPUT_FORMAT", "auto")
        .env("INPUT_SCOPE", "changed")
        .env("INPUT_PROFILE", "default")
        .env("INPUT_FAIL_ON_REVIEW", "definitely")
        .output()
        .expect("run helper");

    assert!(
        output.status.success(),
        "deferred helper failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let args = fs::read_to_string(capture).expect("read captured args");
    for expected in [
        "--fail-on-review",
        "definitely",
        "--scope",
        "changed",
        "--profile",
        "default",
        "--sarif-output",
        "repopilot-review.sarif",
        "--format",
        "json",
    ] {
        assert!(
            args.lines().any(|arg| arg == expected),
            "missing {expected}"
        );
    }

    let outputs = fs::read_to_string(github_output).expect("read github outputs");
    for expected in [
        "review_json_file=repopilot-review.json",
        "review_sarif_file=repopilot-review.sarif",
        "sarif_file=repopilot-review.sarif",
        "findings_count=1",
        "signals_count=1",
        "gate_result=failed",
        "conclusion=failed",
        "exit_code=1",
    ] {
        assert!(outputs.contains(expected), "missing output {expected}");
    }
    assert!(temp.path().join("repopilot-review.json").is_file());
    assert!(temp.path().join("repopilot-review.sarif").is_file());
    assert!(String::from_utf8_lossy(&output.stdout).contains("::warning file=src/auth.rs"));
}
