use std::process::Command;

#[test]
fn agent_workflow_documents_stay_out_of_the_repository_index() {
    let output = Command::new("git")
        .args(["ls-files", "--", "docs/superpowers", ".superpowers"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("git is available for repository hygiene checks");

    assert!(output.status.success(), "git ls-files failed");
    assert!(
        output.stdout.is_empty(),
        "agent workflow documents must stay local, found tracked paths: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
