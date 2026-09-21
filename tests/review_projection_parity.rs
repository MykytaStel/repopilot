use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn review_projections_share_canonical_proof_and_evidence() {
    let temp = tempdir().expect("tempdir");
    init_repo(temp.path());
    write(temp.path(), "src/lib.rs", "pub fn before() {}\n");
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-qm", "base"]);
    write(
        temp.path(),
        "src/lib.rs",
        "pub fn before() {}\npub fn after() {}\n",
    );

    let artifacts = tempdir().expect("artifact tempdir");
    let json_path = artifacts.path().join("review.json");
    let sarif_path = artifacts.path().join("review.sarif");
    let output = repopilot()
        .args([
            "review",
            ".",
            "--format",
            "json",
            "--output",
            json_path.to_str().expect("json path"),
            "--sarif-output",
            sarif_path.to_str().expect("sarif path"),
            "--no-progress",
        ])
        .current_dir(temp.path())
        .output()
        .expect("run JSON review");
    assert!(
        output.status.success(),
        "review failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&fs::read(&json_path).expect("read JSON"))
        .expect("valid review JSON");
    let proof = json["change_proof"].clone();
    let evidence = json["evidence"].clone();
    let proof_receipt = json["proof_receipt"].clone();
    assert!(proof.is_object(), "review JSON keeps change_proof");
    assert!(
        evidence.is_object(),
        "review JSON exposes canonical evidence"
    );
    assert!(
        proof_receipt.is_object(),
        "review JSON exposes replayable proof receipt"
    );
    assert_eq!(json["schema_version"], "0.26");
    assert_eq!(json["report"]["kind"], "review");
    assert!(json["merge_readiness"].is_object());
    assert!(json["findings"].is_array());

    let sarif: Value =
        serde_json::from_slice(&fs::read(&sarif_path).expect("read SARIF")).expect("valid SARIF");
    assert_eq!(sarif["runs"][0]["properties"]["changeProof"], proof);
    assert_eq!(sarif["runs"][0]["properties"]["evidence"], evidence);
    assert_eq!(
        sarif["runs"][0]["properties"]["proofReceipt"],
        proof_receipt
    );

    let markdown = run_text_review(temp.path(), "markdown");
    let html = run_text_review(temp.path(), "html");
    let console = run_text_review(temp.path(), "console");
    let class = human_evidence_class(&evidence);
    let scope = evidence_scope_line(&evidence);
    let provenance = evidence["provenance"]
        .as_object()
        .expect("evidence provenance");
    let provenance_prefix = format!(
        "RepoPilot {}, schema {}",
        provenance["analyzer_version"].as_str().expect("version"),
        provenance["report_schema"].as_str().expect("schema")
    );
    assert!(markdown.contains(&format!("**Evidence class:** `{class}`")));
    assert!(markdown.contains(&format!("**Evidence scope:** {scope}")));
    assert!(markdown.contains(&format!("**Evidence provenance:** {provenance_prefix}")));
    assert!(html.contains(&format!("<dt>Evidence class</dt><dd>{class}</dd>")));
    assert!(html.contains(&format!("<dt>Evidence scope</dt><dd>{scope}</dd>")));
    assert!(html.contains(&provenance_prefix));
    assert!(console.contains(&format!("Evidence class: {class}")));
    assert!(console.contains(&format!("Evidence scope: {scope}")));
    assert!(console.contains(&format!("Evidence provenance: {provenance_prefix}")));
}

fn human_evidence_class(evidence: &Value) -> &'static str {
    match evidence["class"].as_str() {
        Some("observation") => "OBSERVATION",
        Some("supported-proof") => "SUPPORTED PROOF",
        Some("suspicion") => "SUSPICION",
        Some("unknown") => "UNKNOWN",
        other => panic!("unexpected evidence class: {other:?}"),
    }
}

fn evidence_scope_line(evidence: &Value) -> String {
    let scope = &evidence["scope"];
    format!(
        "{}; {}/{} file(s) analyzed; {} excluded, {} unsupported ({})",
        scope["scope"].as_str().expect("scope label"),
        scope["analyzed_files"].as_u64().expect("analyzed files"),
        scope["requested_files"].as_u64().expect("requested files"),
        scope["excluded_files"].as_u64().expect("excluded files"),
        scope["unsupported_files"]
            .as_u64()
            .expect("unsupported files"),
        evidence["coverage_status"]
            .as_str()
            .expect("coverage status")
    )
}

fn run_text_review(root: &Path, format: &str) -> String {
    let output = repopilot()
        .args(["review", ".", "--format", format, "--no-progress"])
        .current_dir(root)
        .output()
        .expect("run text review");
    assert!(
        output.status.success(),
        "{format} review failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("UTF-8 review output")
}

fn init_repo(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@repopilot.local"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, content).expect("write fixture");
}

fn git(root: &Path, args: &[&str]) {
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
}
