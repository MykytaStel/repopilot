use serde_json::Value;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn repopilot_bin() -> &'static str {
    env!("CARGO_BIN_EXE_repopilot")
}

fn create_demo_project() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("failed to create temp dir");
    let root = temp.path();

    fs::create_dir_all(root.join("src")).expect("failed to create src dir");

    fs::write(
        root.join("package.json"),
        r#"{
  "name": "demo-repopilot-project",
  "private": true,
  "dependencies": {
    "react": "18.2.0"
  },
  "devDependencies": {
    "typescript": "5.0.0"
  }
}
"#,
    )
    .expect("failed to write package.json");

    fs::write(
        root.join("src").join("index.ts"),
        r#"
export function main(input: string): string {
  if (input.length > 10) {
    return input.toUpperCase();
  }

  return input.trim();
}
"#,
    )
    .expect("failed to write index.ts");

    fs::write(
        root.join("src").join("config.ts"),
        r#"
// Intentional fake token for static-analysis smoke coverage.
export const API_TOKEN = "sk_live_fake_repopilot_test_token_1234567890";
"#,
    )
    .expect("failed to write config.ts");

    temp
}

fn run_ok<I, S>(cwd: &Path, args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run(cwd, args);

    assert!(
        output.status.success(),
        "command failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    output
}

fn run<I, S>(cwd: &Path, args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(repopilot_bin())
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("failed to run repopilot")
}

fn read_non_empty(path: &Path) -> String {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    assert!(
        !content.trim().is_empty(),
        "{} should not be empty",
        path.display()
    );

    content
}

#[test]
fn cli_version_matches_package_version() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");

    let output = run_ok(temp.path(), ["--version"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "version output should contain package version {}\nstdout:\n{}",
        env!("CARGO_PKG_VERSION"),
        stdout
    );
}

#[test]
fn scan_writes_valid_json_report() {
    let project = create_demo_project();
    let output_path = project.path().join("scan.json");

    run_ok(
        project.path(),
        [
            "scan",
            ".",
            "--format",
            "json",
            "--output",
            output_path.to_str().expect("non-utf8 output path"),
        ],
    );

    let content = read_non_empty(&output_path);
    let json: Value = serde_json::from_str(&content).expect("scan output should be valid JSON");

    assert_eq!(json["files_analyzed"].as_u64().unwrap_or_default(), 3);
    assert!(
        json["findings"].is_array(),
        "scan JSON should include findings array"
    );
}

#[test]
fn release_smoke_covers_rule_quality_gate() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");
    let output_path = temp.path().join("eval-rules.json");

    run_ok(
        temp.path(),
        [
            "inspect",
            "eval-rules",
            "--format",
            "json",
            "--output",
            output_path.to_str().expect("non-utf8 output path"),
        ],
    );

    let content = read_non_empty(&output_path);
    let json: Value =
        serde_json::from_str(&content).expect("eval-rules output should be valid JSON");

    assert_eq!(json["missing_findings"], 0);
    assert_eq!(json["unexpected_findings"], 0);
    assert_eq!(json["contract_violations"], 0);
    assert_eq!(json["stable_id_failures"], 0);
    assert_eq!(json["quality_gate_failures"], 0);
}

#[test]
fn release_smoke_covers_baseline_adoption_path() {
    let project = create_demo_project();
    let baseline_path = project.path().join(".repopilot").join("baseline.json");
    let pass_output_path = project.path().join("baseline-pass.json");
    fs::write(
        project.path().join("src").join("config.rs"),
        "const API_KEY: &str = \"abc12345\";\n",
    )
    .expect("failed to write baseline secret file");

    run_ok(
        project.path(),
        [
            "baseline",
            "create",
            ".",
            "--output",
            baseline_path.to_str().expect("non-utf8 baseline path"),
        ],
    );

    let baseline: Value = serde_json::from_str(&read_non_empty(&baseline_path))
        .expect("baseline should be valid JSON");
    assert_eq!(baseline["schema_version"], 1);

    run_ok(
        project.path(),
        [
            "scan",
            ".",
            "--baseline",
            baseline_path.to_str().expect("non-utf8 baseline path"),
            "--fail-on",
            "new-high",
            "--format",
            "json",
            "--output",
            pass_output_path.to_str().expect("non-utf8 output path"),
        ],
    );

    let pass_report: Value = serde_json::from_str(&read_non_empty(&pass_output_path))
        .expect("scan output should be valid JSON");
    assert_eq!(pass_report["baseline"]["new_findings"], 0);

    fs::write(
        project.path().join("src").join("creds.rs"),
        "const API_KEY: &str = \"abc12345\";\n",
    )
    .expect("failed to write new secret file");

    let failed = run(
        project.path(),
        [
            "scan",
            ".",
            "--baseline",
            baseline_path.to_str().expect("non-utf8 baseline path"),
            "--fail-on",
            "new-high",
        ],
    );

    assert!(
        !failed.status.success(),
        "new high finding should fail the baseline gate\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&failed.stdout),
        String::from_utf8_lossy(&failed.stderr)
    );
    assert!(String::from_utf8_lossy(&failed.stdout).contains("CI gate: failed (new-high)"));
}

#[test]
fn stable_ai_context_writes_llm_ready_markdown() {
    let project = create_demo_project();
    let output_path = project.path().join("ai-context.md");

    run_ok(
        project.path(),
        [
            "ai",
            "context",
            ".",
            "--focus",
            "security",
            "--budget",
            "2k",
            "--output",
            output_path.to_str().expect("non-utf8 output path"),
        ],
    );

    let content = read_non_empty(&output_path);

    assert!(
        content.contains("RepoPilot") || content.contains("AI Context"),
        "ai context output should identify RepoPilot context\n{}",
        content
    );

    assert!(
        content.contains("security")
            || content.contains("Security")
            || content.contains("API_TOKEN")
            || content.contains("token"),
        "ai context output should include security-focused context\n{}",
        content
    );

    // The handoff now folds in the prioritized plan that `ai plan` used to emit.
    assert!(
        content.contains("Remediation Plan") || content.contains("P0"),
        "ai context should embed the prioritized remediation plan\n{}",
        content
    );
}

#[test]
fn inspect_knowledge_writes_valid_json() {
    let project = create_demo_project();
    let output_path = project.path().join("knowledge.json");

    run_ok(
        project.path(),
        [
            "inspect",
            "knowledge",
            "--section",
            "languages",
            "--format",
            "json",
            "--output",
            output_path.to_str().expect("non-utf8 output path"),
        ],
    );

    let content = read_non_empty(&output_path);
    let json: Value =
        serde_json::from_str(&content).expect("knowledge output should be valid JSON");

    assert!(
        json["languages"].is_array(),
        "knowledge JSON should include languages array\n{}",
        content
    );
}

#[test]
fn inspect_explain_writes_valid_json() {
    let project = create_demo_project();
    let output_path = project.path().join("explain.json");

    run_ok(
        project.path(),
        [
            "inspect",
            "explain",
            "src/index.ts",
            "--format",
            "json",
            "--output",
            output_path.to_str().expect("non-utf8 output path"),
        ],
    );

    let content = read_non_empty(&output_path);
    let json: Value = serde_json::from_str(&content).expect("explain output should be valid JSON");

    assert!(
        json["source"].is_object(),
        "explain JSON should include source object\n{}",
        content
    );
    assert!(
        json["context"].is_object(),
        "explain JSON should include context object\n{}",
        content
    );
}
