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
fn action_supports_doctor_as_markdown_auto_command() {
    let action_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("action.yml");
    let action = fs::read_to_string(action_path).expect("read action.yml");

    assert!(action.contains("scan | review | compare | doctor | ai-context"));
    assert!(action.contains("review/compare/doctor"));

    let helper_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/repopilot-action.sh");
    let helper = fs::read_to_string(helper_path).expect("read action helper");
    assert!(helper.contains("doctor) RUN_ARGS=(doctor \"$PATH_INPUT\") ;;"));
    assert!(helper.contains("SARIF output and upload are only supported by 'scan'"));
    assert!(helper.contains("GITHUB_STEP_SUMMARY"));
    assert!(!helper.contains("ai-context|vibe"));
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
        .env("PATH", format!("{}:{}", fake_bin.display(), env!("PATH")))
        .env("CAPTURE_ARGS", &capture)
        .env("GITHUB_STEP_SUMMARY", &step_summary)
        .env("INPUT_COMMAND", "doctor")
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
