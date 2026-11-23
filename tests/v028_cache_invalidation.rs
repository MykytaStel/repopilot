use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

const V027_PARSED_ANALYSIS: &str = "tree-sitter-imports-exports-default-rust-lexical-symbols-spans-guarded-syntax-v9-rust-api-field-contract";

#[test]
fn old_warm_caches_rebuild_cpp_edges_and_rust_enum_facts() {
    let temp = tempdir().expect("temporary repo");
    let root = temp.path();
    prepare_repository(root);

    let first = scan(root);
    assert!(top_dependency_exists(&first, "src/api.hpp"), "{first:#}");
    downgrade_caches_to_v027(root);

    let rebuilt = scan(root);
    assert_ne!(rebuilt["context_graph_cache"]["status"], "hit");
    assert!(
        top_dependency_exists(&rebuilt, "src/api.hpp"),
        "{rebuilt:#}"
    );
    assert_rebuilt_parsed_cache(root);
}

fn prepare_repository(root: &Path) {
    init_repo(root);
    write(
        root,
        "src/app.cpp",
        "#include \"api.hpp\"\nint main() { return api_value(); }\n",
    );
    write(root, "src/api.hpp", "#pragma once\nint api_value();\n");
    write(
        root,
        "src/lib.rs",
        "mod api;\npub fn run() { let _ = api::State::Ready; }\n",
    );
    write(root, "src/api.rs", "pub enum State { Ready }\n");
    commit_all(root);

    write(
        root,
        "src/app.cpp",
        "// changed so its parsed import facts are cached\n#include \"api.hpp\"\nint main() { return api_value(); }\n",
    );
    write(
        root,
        "src/lib.rs",
        "// changed so its enum-use facts are cached\nmod api;\npub fn run() { let _ = api::State::Ready; }\n",
    );
    write(
        root,
        "src/api.rs",
        "// changed so its enum-declaration facts are cached\npub enum State { Ready }\n",
    );
}

fn downgrade_caches_to_v027(root: &Path) {
    let cache_dir = root.join(".repopilot/cache");
    let parsed_path = cache_dir.join("parsed_facts_v2.json");
    let mut parsed = read_json(&parsed_path);
    assert!(contains_cpp_include(&parsed));
    assert!(contains_rust_enum_facts(&parsed));

    parsed["analysis_version"] = Value::String(V027_PARSED_ANALYSIS.to_string());
    for entry in parsed["entries"].as_array_mut().expect("parsed entries") {
        if matches!(entry["language"].as_str(), Some("C++" | "C/C++ Header")) {
            entry["imports"] = json!([]);
            entry["import_spans"] = json!({});
        }
        if entry["language"] == "Rust"
            && let Some(contracts) = entry["javascript_symbols"]["rust_contracts"].as_object_mut()
        {
            contracts.remove("public_enums");
            contracts.remove("public_enum_variants");
            contracts.remove("enum_variant_uses");
        }
    }
    write_json(&parsed_path, &parsed);

    let context_path = cache_dir.join("repo_context.json");
    let mut context = read_json(&context_path);
    context["resolver_version"] = Value::String("context-state-v1".to_string());
    write_json(&context_path, &context);
}

fn assert_rebuilt_parsed_cache(root: &Path) {
    let parsed_path = root.join(".repopilot/cache/parsed_facts_v2.json");
    let reparsed = read_json(&parsed_path);
    assert_ne!(reparsed["analysis_version"], V027_PARSED_ANALYSIS);
    assert!(contains_cpp_include(&reparsed));
    assert!(contains_rust_enum_facts(&reparsed));
}

fn contains_cpp_include(cache: &Value) -> bool {
    cache["entries"].as_array().is_some_and(|entries| {
        entries.iter().any(|entry| {
            matches!(entry["language"].as_str(), Some("C++" | "C/C++ Header"))
                && entry["imports"]
                    .as_array()
                    .is_some_and(|imports| imports.iter().any(|path| path == "api.hpp"))
        })
    })
}

fn contains_rust_enum_facts(cache: &Value) -> bool {
    cache["entries"].as_array().is_some_and(|entries| {
        entries.iter().any(|entry| {
            entry["language"] == "Rust"
                && entry["javascript_symbols"]["rust_contracts"]["public_enum_variants"]
                    .as_array()
                    .is_some_and(|variants| !variants.is_empty())
        }) && entries.iter().any(|entry| {
            entry["language"] == "Rust"
                && entry["javascript_symbols"]["rust_contracts"]["enum_variant_uses"]
                    .as_array()
                    .is_some_and(|uses| !uses.is_empty())
        })
    })
}

fn top_dependency_exists(report: &Value, path: &str) -> bool {
    report["context_graph_summary"]["top_dependencies"]
        .as_array()
        .is_some_and(|dependencies| {
            dependencies
                .iter()
                .any(|dependency| dependency["path"] == path)
        })
}

fn scan(root: &Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["scan", ".", "--changed", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("scan command runs");
    assert!(
        output.status.success(),
        "scan failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("scan JSON")
}

fn init_repo(root: &Path) {
    for args in [
        &["init", "-q", "--initial-branch=main"][..],
        &["config", "user.email", "test@example.invalid"],
        &["config", "user.name", "RepoPilot cache test"],
    ] {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git command runs");
        assert!(output.status.success(), "git {args:?}");
    }
}

fn commit_all(root: &Path) {
    for args in [&["add", "."][..], &["commit", "-qm", "initial"]] {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git command runs");
        assert!(output.status.success(), "git {args:?}");
    }
}

fn write(root: &Path, path: &str, contents: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent path")).expect("create parent");
    fs::write(path, contents).expect("write source");
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("read JSON file")).expect("parse JSON")
}

fn write_json(path: &Path, value: &Value) {
    fs::write(
        path,
        serde_json::to_vec_pretty(value).expect("serialize JSON"),
    )
    .expect("write JSON");
}
