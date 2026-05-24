use serde_json::Value;
use std::fs;
use std::process::{Command, Output};
use tempfile::TempDir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

fn run_repopilot(args: &[&str]) -> Output {
    repopilot().args(args).output().expect("run repopilot")
}

fn fixture_project() -> TempDir {
    let dir = tempfile::tempdir().expect("create fixture project");
    fs::create_dir_all(dir.path().join("src")).expect("create src dir");
    fs::write(
        dir.path().join("src/lib.rs"),
        "pub mod domain;\npub fn answer() -> i32 { domain::value() }\n",
    )
    .expect("write lib source");
    fs::write(dir.path().join("src/domain.rs"), "pub fn value() -> i32 { 42 }\n")
        .expect("write domain source");
    dir
}

#[test]
fn given_small_project_when_scan_runs_then_finalization_artifacts_are_present() {
    // Given
    let project = fixture_project();
    let output_path = project.path().join("scan.json");
    let project_path = project.path().to_string_lossy().to_string();
    let output_arg = output_path.to_string_lossy().to_string();

    // When
    let output = run_repopilot(&[
        "scan",
        &project_path,
        "--format",
        "json",
        "--timing",
        "--output",
        &output_arg,
    ]);

    // Then
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("report finalization"),
        "timing stderr should mention report finalization, got:\n{stderr}"
    );

    let report: Value = serde_json::from_str(
        &fs::read_to_string(&output_path).expect("scan report should be written"),
    )
    .expect("scan report should be valid JSON");

    assert!(
        report["context_graph_summary"].is_object(),
        "finalized scan should include context_graph_summary: {report:#?}"
    );
    assert!(
        report["scan_timings"]["report_finalization_us"]
            .as_u64()
            .is_some(),
        "finalized scan should include report_finalization_us timing: {report:#?}"
    );
}
