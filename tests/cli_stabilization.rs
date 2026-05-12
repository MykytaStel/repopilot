use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn repopilot_bin() -> &'static str {
    env!("CARGO_BIN_EXE_repopilot")
}

fn run(args: &[&str], cwd: &Path) -> Output {
    Command::new(repopilot_bin())
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("failed to run repopilot")
}

fn run_ok(args: &[&str], cwd: &Path) -> Output {
    let output = run(args, cwd);
    assert!(
        output.status.success(),
        "command failed\nargs: {:?}\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn create_project() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("temp dir");
    fs::create_dir_all(temp.path().join("src")).expect("src dir");
    fs::create_dir_all(temp.path().join("tests")).expect("tests dir");
    fs::write(
        temp.path().join("src/lib.rs"),
        "// TODO: tracked by CLI stabilization test\npub fn token() -> &'static str { \"sk_live_fake_repopilot_test_token_123456\" }\n",
    )
    .expect("source file");
    fs::write(
        temp.path().join("tests/lib_test.rs"),
        "#[test]\nfn token_exists() { assert!(demo::token().contains(\"sk_live\")); }\n",
    )
    .expect("test file");
    temp
}

#[test]
fn top_level_help_shows_stable_command_surface() {
    let temp = tempfile::tempdir().expect("temp dir");
    let output = run_ok(&["--help"], temp.path());
    let help = stdout(&output);

    for command in [
        "baseline", "compare", "scan", "review", "ai", "inspect", "init", "doctor",
    ] {
        assert!(help.contains(command), "help should show {command}\n{help}");
    }

    for hidden in ["vibe", "harden", "prompt", "explain", "knowledge"] {
        assert!(
            !help.contains(&format!("  {hidden}")),
            "top-level help should hide legacy command {hidden}\n{help}"
        );
    }
}

#[test]
fn scan_and_review_help_have_flag_descriptions() {
    let temp = tempfile::tempdir().expect("temp dir");
    let scan_help = stdout(&run_ok(&["scan", "--help"], temp.path()));
    let review_help = stdout(&run_ok(&["review", "--help"], temp.path()));

    assert!(scan_help.contains("Path to project, folder, or file to scan"));
    assert!(scan_help.contains("Write report to a file instead of stdout"));
    assert!(scan_help.contains("Scan each detected workspace package separately"));

    assert!(review_help.contains("Path to project, folder, or file to review"));
    assert!(review_help.contains("Base Git ref for branch/CI review"));
    assert!(review_help.contains("Exit with code 1 when in-diff findings"));
}

#[test]
fn grouped_ai_commands_and_legacy_commands_both_work() {
    let project = create_project();

    let context = run_ok(
        &[
            "ai", "context", ".", "--focus", "security", "--budget", "2k",
        ],
        project.path(),
    );
    let legacy_vibe = run_ok(
        &["vibe", ".", "--focus", "security", "--budget", "2k"],
        project.path(),
    );

    assert!(stdout(&context).contains("RepoPilot Vibe Check"));
    assert!(stdout(&legacy_vibe).contains("RepoPilot Vibe Check"));

    let plan = run_ok(&["ai", "plan", ".", "--budget", "2k"], project.path());
    let legacy_harden = run_ok(&["harden", ".", "--budget", "2k"], project.path());
    assert!(stdout(&plan).contains("RepoPilot Harden Plan"));
    assert!(stdout(&legacy_harden).contains("RepoPilot Harden Plan"));

    let prompt = run_ok(&["ai", "prompt", ".", "--budget", "2k"], project.path());
    let legacy_prompt = run_ok(&["prompt", ".", "--budget", "2k"], project.path());
    assert!(stdout(&prompt).contains("RepoPilot Remediation Prompt"));
    assert!(stdout(&legacy_prompt).contains("RepoPilot Remediation Prompt"));
}

#[test]
fn inspect_commands_and_legacy_commands_both_work() {
    let project = create_project();

    let inspect = run_ok(
        &[
            "inspect",
            "explain",
            "src/lib.rs",
            "--format",
            "json",
            "--rule",
            "language.rust.panic-risk",
            "--signal",
            "rust.unwrap",
        ],
        project.path(),
    );
    let legacy = run_ok(
        &[
            "explain",
            "src/lib.rs",
            "--format",
            "json",
            "--rule",
            "language.rust.panic-risk",
            "--signal",
            "rust.unwrap",
        ],
        project.path(),
    );

    let inspect_json: Value = serde_json::from_slice(&inspect.stdout).expect("inspect json");
    let legacy_json: Value = serde_json::from_slice(&legacy.stdout).expect("legacy json");
    assert_eq!(inspect_json["context"], legacy_json["context"]);

    let knowledge = run_ok(
        &[
            "inspect",
            "knowledge",
            "--section",
            "rules",
            "--format",
            "json",
        ],
        project.path(),
    );
    let legacy_knowledge = run_ok(
        &["knowledge", "--section", "rules", "--format", "json"],
        project.path(),
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&knowledge.stdout).expect("knowledge json")["summary"],
        serde_json::from_slice::<Value>(&legacy_knowledge.stdout).expect("legacy knowledge json")["summary"]
    );
}

#[test]
fn exit_codes_distinguish_findings_usage_and_runtime_errors() {
    let project = create_project();

    let threshold = run(
        &["scan", ".", "--fail-on", "low", "--format", "json"],
        project.path(),
    );
    assert_eq!(threshold.status.code(), Some(1));

    let usage = run(&["review", ".", "--head", "HEAD"], project.path());
    assert_eq!(usage.status.code(), Some(2));

    let runtime = run(&["scan", "missing-path"], project.path());
    assert_eq!(runtime.status.code(), Some(3));
}

#[test]
fn self_audit_stays_clean_at_high_severity() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = run_ok(&["scan", ".", "--min-severity", "high"], repo);
    let report = stdout(&output);

    assert!(
        report.contains("Findings: 0") || report.contains("Findings: 0 "),
        "self-audit high severity should stay clean\n{report}"
    );
}
