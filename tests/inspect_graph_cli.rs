use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn inspect_graph_renders_json_contract() {
    let temp = tempdir().expect("temp dir");
    write_graph_fixture(temp.path());

    let output = repopilot()
        .args(["inspect", "graph", ".", "--format", "json"])
        .current_dir(temp.path())
        .output()
        .expect("inspect graph json");

    assert!(
        output.status.success(),
        "inspect graph failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("graph json");

    assert_eq!(json["kind"], "context-graph");
    assert_eq!(json["context_graph_summary"]["files"], 2);
    assert_eq!(json["context_graph_cache"]["status"], "write");
    assert!(
        temp.path()
            .join(".repopilot/cache/repo_context.json")
            .is_file()
    );
}

#[test]
fn inspect_graph_renders_markdown_contract() {
    let temp = tempdir().expect("temp dir");
    write_graph_fixture(temp.path());

    let output = repopilot()
        .args(["inspect", "graph", ".", "--format", "markdown"])
        .current_dir(temp.path())
        .output()
        .expect("inspect graph markdown");

    assert!(output.status.success());
    let markdown = String::from_utf8_lossy(&output.stdout);

    assert!(markdown.contains("# RepoPilot Context Risk Graph"));
    assert!(markdown.contains("## Top Dependencies"));
    assert!(markdown.contains("## Risky Clusters"));
}

fn write_graph_fixture(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "mod a;\npub fn lib() { a::a(); }\n",
    )
    .expect("write lib");
    fs::write(root.join("src/a.rs"), "pub fn a() {}\n").expect("write a");
}
