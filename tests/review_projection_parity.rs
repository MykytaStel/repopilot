use repopilot::baseline::diff::BaselineStatus;
use repopilot::findings::record::FindingRecord;
use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::report::schema::ReviewJsonFinding;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn review_json_finding_uses_the_shared_explanation_card() {
    let finding = Finding {
        id: "rule.example:src/lib.rs:1".to_string(),
        rule_id: "rule.example".to_string(),
        title: "Example".to_string(),
        description: "An example structural signal.".to_string(),
        recommendation: "Review the example.".to_string(),
        category: FindingCategory::Architecture,
        severity: Severity::Medium,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: "src/lib.rs".into(),
            line_start: 1,
            line_end: None,
            snippet: "pub fn example() {}".to_string(),
        }],
        ..Default::default()
    };
    let projected = ReviewJsonFinding {
        record: FindingRecord::new(&finding),
        in_diff: true,
        baseline_status: Some(BaselineStatus::New),
    };
    let value = serde_json::to_value(projected).expect("review finding JSON");

    assert_eq!(
        value["decision"]["explanation"]["claim"],
        "An example structural signal."
    );
    assert_eq!(
        value["decision"]["explanation"]["evidence_basis"]["scope"],
        "file"
    );
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
    let decision = json["decision"].clone();
    let proof_receipt = json["proof_receipt"].clone();
    assert!(proof.is_object(), "review JSON keeps change_proof");
    assert!(
        decision.is_object(),
        "review JSON exposes canonical decision"
    );
    assert!(decision["next_action"].is_string());
    assert!(
        evidence.is_object(),
        "review JSON exposes canonical evidence"
    );
    assert!(
        proof_receipt.is_object(),
        "review JSON exposes replayable proof receipt"
    );
    let coverage_limits = evidence["coverage_limits"]
        .as_array()
        .expect("review JSON exposes structured coverage limits");
    assert!(coverage_limits.iter().any(|limit| {
        limit["code"] == "verification-not-configured"
            && limit["message"] == "Verification checks are not configured."
    }));
    assert_eq!(json["schema_version"], "0.26");
    assert_eq!(json["report"]["kind"], "review");
    assert!(json["merge_readiness"].is_object());
    assert!(json["findings"].is_array());

    let sarif: Value =
        serde_json::from_slice(&fs::read(&sarif_path).expect("read SARIF")).expect("valid SARIF");
    assert_eq!(sarif["runs"][0]["properties"]["changeProof"], proof);
    assert_eq!(sarif["runs"][0]["properties"]["decision"], decision);
    assert_eq!(sarif["runs"][0]["properties"]["evidence"], evidence);
    assert_eq!(
        sarif["runs"][0]["properties"]["proofReceipt"],
        proof_receipt
    );

    let markdown = run_text_review(temp.path(), "markdown");
    let html = run_text_review(temp.path(), "html");
    let console = run_text_review(temp.path(), "console");
    let console_full = run_text_review_with(temp.path(), "console", &["--detail", "full"]);
    let class = human_evidence_class(&evidence);
    let decision_label = decision["verdict"].as_str().expect("decision verdict");
    let decision_action = decision["next_action"].as_str().expect("decision action");
    let scope = evidence_scope_line(&evidence);
    let provenance = evidence["provenance"]
        .as_object()
        .expect("evidence provenance");
    let provenance_prefix = format!(
        "RepoPilot {}, schema {}",
        provenance["analyzer_version"].as_str().expect("version"),
        provenance["report_schema"].as_str().expect("schema")
    );
    assert!(markdown.contains(&format!("**Decision:** `{decision_label}`")));
    assert!(markdown.contains(decision_action));
    assert!(markdown.contains(&format!("**Evidence class:** `{class}`")));
    assert!(markdown.contains(&format!("**Evidence scope:** {scope}")));
    assert!(markdown.contains(&format!("**Evidence provenance:** {provenance_prefix}")));
    assert!(html.contains("<strong>Decision:</strong>"));
    assert!(html.contains(decision_label));
    assert!(html.contains(decision_action));
    assert!(html.contains(&format!("<dt>Evidence class</dt><dd>{class}</dd>")));
    assert!(html.contains(&format!("<dt>Evidence scope</dt><dd>{scope}</dd>")));
    assert!(html.contains(&provenance_prefix));
    assert!(console.contains(&format!("Decision: {decision_label}")));
    assert!(console.contains(decision_action));
    assert!(console.contains(&format!("Evidence scope: {scope}")));
    assert!(!console.contains("Legacy merge readiness"));
    assert!(console_full.contains(&format!("Decision: {decision_label}")));
    assert!(console_full.contains(&format!("Evidence class: {class}")));
    assert!(console_full.contains(&format!("Evidence scope: {scope}")));
    assert!(console_full.contains(&format!("Evidence provenance: {provenance_prefix}")));
    assert!(console_full.contains("Legacy merge readiness"));
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
    let status = evidence["coverage_status"]
        .as_str()
        .expect("coverage status");
    let limits = evidence["coverage_limits"]
        .as_array()
        .expect("coverage limits")
        .iter()
        .filter_map(|limit| limit["message"].as_str())
        .collect::<Vec<_>>();
    let status = if limits.is_empty() {
        status.to_string()
    } else {
        format!("{status}: {}", limits.join("; "))
    };
    format!(
        "{}; {}/{} file(s) analyzed; {} excluded, {} unsupported ({status})",
        scope["scope"].as_str().expect("scope label"),
        scope["analyzed_files"].as_u64().expect("analyzed files"),
        scope["requested_files"].as_u64().expect("requested files"),
        scope["excluded_files"].as_u64().expect("excluded files"),
        scope["unsupported_files"]
            .as_u64()
            .expect("unsupported files"),
    )
}

fn run_text_review(root: &Path, format: &str) -> String {
    run_text_review_with(root, format, &[])
}

fn run_text_review_with(root: &Path, format: &str, extra: &[&str]) -> String {
    let output = repopilot()
        .args(["review", ".", "--format", format, "--no-progress"])
        .args(extra)
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
