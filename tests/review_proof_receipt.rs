use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn review_json_and_sarif_publish_the_same_proof_receipt() {
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
    let sarif: Value =
        serde_json::from_slice(&fs::read(&sarif_path).expect("read SARIF")).expect("valid SARIF");
    let receipt = &json["proof_receipt"];

    assert!(receipt.is_object(), "review JSON exposes proof receipt");
    assert_eq!(receipt, &sarif["runs"][0]["properties"]["proofReceipt"]);
    assert_eq!(receipt["proof"], json["change_proof"]);
    assert_eq!(receipt["replay_state"], "matched");
    assert!(receipt["unavailable_inputs"].is_array());
    assert_eq!(
        receipt.get("findings"),
        None,
        "receipt does not aggregate occurrence-level finding records"
    );
}

fn init_repo(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@repopilot.local"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, content).expect("write file");
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
