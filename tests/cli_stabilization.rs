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

    for removed in ["vibe", "harden", "prompt", "explain", "knowledge"] {
        assert!(
            !help.contains(&format!("  {removed}  ")),
            "top-level help should not list removed command {removed} as a subcommand\n{help}"
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
fn grouped_ai_commands_work() {
    let project = create_project();

    let context = run_ok(
        &[
            "ai", "context", ".", "--focus", "security", "--budget", "2k",
        ],
        project.path(),
    );
    assert!(stdout(&context).contains("RepoPilot AI Context"));

    let plan = run_ok(&["ai", "plan", ".", "--budget", "2k"], project.path());
    assert!(stdout(&plan).contains("RepoPilot AI Plan"));

    let prompt = run_ok(&["ai", "prompt", ".", "--budget", "2k"], project.path());
    assert!(stdout(&prompt).contains("RepoPilot Remediation Prompt"));
}

#[test]
fn inspect_commands_work() {
    let project = create_project();

    let explain = run_ok(
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

    let explain_json: Value =
        serde_json::from_slice(&explain.stdout).expect("inspect explain json");
    assert!(explain_json["context"].is_object());

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

    let knowledge_json: Value =
        serde_json::from_slice::<Value>(&knowledge.stdout).expect("knowledge json");
    assert!(knowledge_json["summary"].is_object());
}

#[test]
fn legacy_commands_are_removed_from_executable_surface() {
    let project = create_project();

    for command in ["vibe", "harden", "prompt", "explain", "knowledge"] {
        let output = run(&[command, "."], project.path());
        assert_eq!(
            output.status.code(),
            Some(2),
            "legacy command {command} should be rejected\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn exit_codes_distinguish_findings_usage_and_runtime_errors() {
    let project = create_project();

    let threshold = run(
        &[
            "scan",
            ".",
            "--fail-on",
            "low",
            "--format",
            "json",
            "--profile",
            "strict",
        ],
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
    let output = run_ok(&["scan", ".", "--format", "json"], repo);
    let json: Value = serde_json::from_slice(&output.stdout).expect("json output from self-audit");
    let high_or_higher = json["findings"]
        .as_array()
        .map(|findings| {
            findings
                .iter()
                .filter(|finding| matches!(finding["severity"].as_str(), Some("HIGH" | "CRITICAL")))
                .count()
        })
        .unwrap_or(usize::MAX);
    let p0 = json["risk_summary"]["counts"]["p0"].as_u64().unwrap_or(1);
    let p1 = json["risk_summary"]["counts"]["p1"].as_u64().unwrap_or(1);
    let p2 = json["risk_summary"]["counts"]["p2"]
        .as_u64()
        .unwrap_or(u64::MAX);
    assert_eq!(
        high_or_higher,
        0,
        "self-audit high severity should stay clean\n{}",
        serde_json::to_string_pretty(&json).unwrap_or_default()
    );
    assert_eq!(p0, 0, "self-audit should not produce P0 findings");
    assert!(
        p1 <= 1,
        "self-audit should keep default-visible P1 findings within the product signal budget, got {p1}"
    );
    assert!(
        p2 <= 135,
        "self-audit P2 noise should stay within the signal-quality budget, got {p2}\n{}",
        serde_json::to_string_pretty(&json).unwrap_or_default()
    );
}
