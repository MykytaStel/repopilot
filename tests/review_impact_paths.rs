//! End-to-end proof that `review` traces bounded-depth dependency impact
//! paths through the real CLI-facing JSON, additively alongside (and without
//! disturbing) the existing one-hop `blast_radius` field.

use repopilot::config::model::RepoPilotConfig;
use repopilot::review::build_review_report;
use repopilot::review::render::render_json;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn impact_paths_split_direct_and_transitive_dependents_by_default_depth() {
    let temp = TempDir::new().expect("failed to create temp dir");
    init_repo(temp.path());
    // c.ts -> b.ts -> a.ts (an import chain two hops deep).
    write_file(&temp, "src/a.ts", "export const a = 1;\n");
    write_file(
        &temp,
        "src/b.ts",
        "import { a } from \"./a\";\nexport const b = a;\n",
    );
    write_file(
        &temp,
        "src/c.ts",
        "import { b } from \"./b\";\nexport const c = b;\n",
    );
    commit_all(temp.path(), "initial");

    write_file(&temp, "src/a.ts", "export const a = 2;\n");

    let json = run_review_json(temp.path(), &RepoPilotConfig::default());
    let files = json["impact_paths"]["files"]
        .as_array()
        .expect("impact_paths.files array");
    let a_impact = files
        .iter()
        .find(|file| file["path"] == "src/a.ts")
        .expect("impact entry for src/a.ts");

    let direct = a_impact["direct_dependents"]
        .as_array()
        .expect("direct_dependents array");
    let transitive = a_impact["transitive_dependents"]
        .as_array()
        .expect("transitive_dependents array");
    assert!(direct.iter().any(|path| path == "src/b.ts"));
    assert!(!direct.iter().any(|path| path == "src/c.ts"));
    assert!(transitive.iter().any(|path| path == "src/c.ts"));

    assert_eq!(json["review"]["impact_path_depth"], 3);
    assert_eq!(json["review"]["affected_files"], 2);
    assert_eq!(json["review"]["affected_directories"], 1);
}

#[test]
fn impact_path_depth_one_excludes_transitive_dependents() {
    let temp = TempDir::new().expect("failed to create temp dir");
    init_repo(temp.path());
    write_file(&temp, "src/a.ts", "export const a = 1;\n");
    write_file(
        &temp,
        "src/b.ts",
        "import { a } from \"./a\";\nexport const b = a;\n",
    );
    write_file(
        &temp,
        "src/c.ts",
        "import { b } from \"./b\";\nexport const c = b;\n",
    );
    commit_all(temp.path(), "initial");

    write_file(&temp, "src/a.ts", "export const a = 2;\n");

    let mut config = RepoPilotConfig::default();
    config.review.impact_path_depth = 1;

    let json = run_review_json(temp.path(), &config);
    let files = json["impact_paths"]["files"]
        .as_array()
        .expect("impact_paths.files array");
    let a_impact = files
        .iter()
        .find(|file| file["path"] == "src/a.ts")
        .expect("impact entry for src/a.ts");

    assert!(
        a_impact["transitive_dependents"]
            .as_array()
            .expect("transitive_dependents array")
            .is_empty()
    );
    assert_eq!(json["review"]["affected_files"], 1);
}

#[test]
fn impact_paths_are_empty_for_a_leaf_change_and_deterministic_across_runs() {
    let temp = TempDir::new().expect("failed to create temp dir");
    init_repo(temp.path());
    write_file(&temp, "src/b.ts", "export const b = 1;\n");
    write_file(
        &temp,
        "src/a.ts",
        "import { b } from \"./b\";\nexport const a = b;\n",
    );
    commit_all(temp.path(), "initial");

    write_file(
        &temp,
        "src/a.ts",
        "import { b } from \"./b\";\nexport const a = b + 1;\n",
    );

    let config = RepoPilotConfig::default();
    let first = run_review_json(temp.path(), &config);
    let second = run_review_json(temp.path(), &config);

    assert_eq!(first["impact_paths"], second["impact_paths"]);
    assert!(
        first["impact_paths"]["files"]
            .as_array()
            .expect("files array")
            .is_empty()
    );
    assert_eq!(first["review"]["affected_files"], 0);
}

fn run_review_json(root: &Path, config: &RepoPilotConfig) -> Value {
    let summary = scan_path_with_config(root, &ScanConfig::default()).expect("failed to scan");
    let report = build_review_report(summary, root, None, None, None, config)
        .expect("failed to build review");
    let output = render_json(&report, None).expect("failed to render review json");

    serde_json::from_str(&output).expect("expected JSON output")
}

fn write_file(temp: &TempDir, relative_path: &str, content: &str) {
    let path = temp.path().join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create parent dir");
    }
    fs::write(path, content).expect("failed to write file");
}

fn init_repo(root: &Path) {
    git(root, &["init"]);
    git(root, &["config", "user.email", "repopilot@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(root, &["commit", "-m", message]);
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("failed to run git");

    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}
