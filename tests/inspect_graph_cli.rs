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

#[test]
fn inspect_graph_rejects_unsupported_formats() {
    let temp = tempdir().expect("temp dir");
    write_graph_fixture(temp.path());

    for format in ["html", "sarif"] {
        let output = repopilot()
            .args(["inspect", "graph", ".", "--format", format])
            .current_dir(temp.path())
            .output()
            .expect("inspect graph unsupported format");

        assert!(
            !output.status.success(),
            "inspect graph --format {format} should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("invalid value") || stderr.contains("supports only"),
            "expected clear usage error for {format}, got:\n{stderr}"
        );
        assert!(
            stderr.contains("console") && stderr.contains("markdown") && stderr.contains("json"),
            "expected supported formats in error for {format}, got:\n{stderr}"
        );
    }
}

#[test]
fn inspect_graph_cache_hits_on_second_run() {
    let temp = tempdir().expect("temp dir");
    write_graph_fixture(temp.path());

    let first = inspect_graph_json(temp.path());
    assert_eq!(first["context_graph_cache"]["status"], "write");

    let second = inspect_graph_json(temp.path());
    assert_eq!(second["context_graph_cache"]["status"], "hit");
}

#[test]
fn inspect_graph_cache_rebuilds_when_import_changes() {
    let temp = tempdir().expect("temp dir");
    write_graph_fixture(temp.path());
    fs::write(temp.path().join("src/b.rs"), "pub fn b() {}\n").expect("write b");

    let first = inspect_graph_json(temp.path());
    assert_eq!(first["context_graph_cache"]["status"], "write");

    fs::write(
        temp.path().join("src/lib.rs"),
        "use crate::b;\npub fn lib() { b::b(); }\n",
    )
    .expect("rewrite lib");

    let changed = inspect_graph_json(temp.path());
    assert_eq!(changed["context_graph_cache"]["status"], "write");
    assert!(top_dependencies_include(&changed, "src/b.rs"));
}

#[test]
fn inspect_graph_cache_rebuilds_when_source_file_is_added_or_deleted() {
    let temp = tempdir().expect("temp dir");
    write_graph_fixture(temp.path());

    let first = inspect_graph_json(temp.path());
    assert_eq!(first["context_graph_cache"]["status"], "write");

    fs::write(temp.path().join("src/b.rs"), "pub fn b() {}\n").expect("write b");
    let added = inspect_graph_json(temp.path());
    assert_eq!(added["context_graph_cache"]["status"], "write");
    assert_eq!(added["context_graph_summary"]["files"], 3);

    fs::remove_file(temp.path().join("src/a.rs")).expect("delete a");
    let deleted = inspect_graph_json(temp.path());
    assert_eq!(deleted["context_graph_cache"]["status"], "write");
    assert_eq!(deleted["context_graph_summary"]["files"], 2);
}

fn inspect_graph_json(root: &Path) -> Value {
    let output = repopilot()
        .args(["inspect", "graph", ".", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("inspect graph json");

    assert!(
        output.status.success(),
        "inspect graph failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("graph json")
}

fn top_dependencies_include(json: &Value, expected_path: &str) -> bool {
    json["context_graph_summary"]["top_dependencies"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|metric| metric["path"] == expected_path)
}

fn write_graph_fixture(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "use crate::a;\npub fn lib() { a::a(); }\n",
    )
    .expect("write lib");
    fs::write(root.join("src/a.rs"), "pub fn a() {}\n").expect("write a");
}
