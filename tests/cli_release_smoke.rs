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
    let output = Command::new(repopilot_bin())
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("failed to run repopilot");

    assert!(
        output.status.success(),
        "command failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    output
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

    assert_eq!(json["files_count"].as_u64().unwrap_or_default(), 3);
    assert!(
        json["findings"].is_array(),
        "scan JSON should include findings array"
    );
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
        content.contains("RepoPilot") || content.contains("Vibe"),
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
}

#[test]
fn stable_ai_plan_writes_prioritized_remediation_plan() {
    let project = create_demo_project();
    let output_path = project.path().join("ai-plan.md");

    run_ok(
        project.path(),
        [
            "ai",
            "plan",
            ".",
            "--focus",
            "all",
            "--budget",
            "4k",
            "--output",
            output_path.to_str().expect("non-utf8 output path"),
        ],
    );

    let content = read_non_empty(&output_path);

    assert!(
        content.contains("P0")
            || content.contains("P1")
            || content.contains("Priority")
            || content.contains("Remediation"),
        "ai plan output should look like a remediation plan\n{}",
        content
    );
}

#[test]
fn stable_ai_prompt_writes_ai_ready_prompt() {
    let project = create_demo_project();
    let output_path = project.path().join("ai-prompt.md");

    run_ok(
        project.path(),
        [
            "ai",
            "prompt",
            ".",
            "--focus",
            "quality",
            "--budget",
            "4k",
            "--output",
            output_path.to_str().expect("non-utf8 output path"),
        ],
    );

    let content = read_non_empty(&output_path);

    assert!(
        content.contains("RepoPilot")
            || content.contains("prompt")
            || content.contains("fix")
            || content.contains("Findings"),
        "prompt output should include AI remediation context\n{}",
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
