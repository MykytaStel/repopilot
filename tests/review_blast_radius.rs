use repopilot::config::model::SecurityBoundarySection;
use repopilot::graph::CouplingGraph;
use repopilot::review::diff::{ChangeStatus, ChangedFile};
use repopilot::review::model::ReviewReport;
use repopilot::review::render::{render_console, render_json};
use repopilot::review::{build_review_report, compute_blast_radius};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use repopilot::scan::types::{ScanArtifacts, ScanMetadata, ScanSummary};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

#[test]
fn review_blast_radius_includes_files_that_import_changed_file() {
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
        "import { a } from \"./a\";\nexport const c = a;\n",
    );
    commit_all(temp.path(), "initial");

    write_file(&temp, "src/a.ts", "export const a = 2;\n");

    let json = run_review_json(temp.path());
    let blast_radius = json["blast_radius"].as_array().expect("blast radius array");

    assert!(blast_radius.iter().any(|path| path == "src/b.ts"));
    assert!(blast_radius.iter().any(|path| path == "src/c.ts"));
    assert!(!blast_radius.iter().any(|path| path == "src/a.ts"));
}

#[test]
fn review_blast_radius_is_empty_for_changed_leaf_file() {
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

    let json = run_review_json(temp.path());

    assert_eq!(json["blast_radius"].as_array().unwrap().len(), 0);
}

#[test]
fn review_blast_radius_excludes_changed_importers() {
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
        "import { a } from \"./a\";\nexport const c = a;\n",
    );
    commit_all(temp.path(), "initial");

    write_file(&temp, "src/a.ts", "export const a = 2;\n");
    write_file(
        &temp,
        "src/b.ts",
        "import { a } from \"./a\";\nexport const b = a + 1;\n",
    );

    let json = run_review_json(temp.path());
    let blast_radius = json["blast_radius"].as_array().expect("blast radius array");

    assert!(blast_radius.iter().any(|path| path == "src/c.ts"));
    assert!(!blast_radius.iter().any(|path| path == "src/b.ts"));
    assert!(!blast_radius.iter().any(|path| path == "src/a.ts"));
}

#[test]
fn blast_radius_is_empty_without_coupling_graph() {
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(&temp, "src/a.ts", "export const a = 1;\n");

    let summary = ScanSummary {
        metadata: ScanMetadata {
            root_path: temp.path().to_path_buf(),
            ..ScanMetadata::default()
        },
        artifacts: ScanArtifacts {
            coupling_graph: None,
            ..ScanArtifacts::default()
        },
        ..ScanSummary::default()
    };
    let changed_files = vec![changed_file("src/a.ts")];

    let blast_radius = compute_blast_radius(&summary, temp.path(), &changed_files);

    assert!(blast_radius.is_empty());
}

#[test]
fn render_console_includes_blast_radius_section_when_present() {
    let temp = TempDir::new().expect("failed to create temp dir");
    write_file(&temp, "src/a.ts", "export const a = 1;\n");
    write_file(
        &temp,
        "src/b.ts",
        "import { a } from \"./a\";\nexport const b = a;\n",
    );

    let report = ReviewReport {
        summary: ScanSummary {
            metadata: ScanMetadata {
                root_path: temp.path().to_path_buf(),
                ..ScanMetadata::default()
            },
            artifacts: ScanArtifacts {
                coupling_graph: Some(CouplingGraph {
                    edges: BTreeMap::new(),
                    nodes: BTreeSet::new(),
                }),
                ..ScanArtifacts::default()
            },
            ..ScanSummary::default()
        },
        repo_root: temp.path().to_path_buf(),
        baseline_path: None,
        changed_files: vec![changed_file("src/a.ts")],
        blast_radius: vec![PathBuf::from("src/b.ts")],
        boundary_signals: vec![],
        findings: vec![],
    };

    let output = render_console(&report, None);

    assert!(output.contains("Blast radius"));
    assert!(output.contains("src/b.ts"));
}

fn run_review_json(root: &Path) -> Value {
    let summary = scan_path_with_config(root, &ScanConfig::default()).expect("failed to scan");
    let report = build_review_report(
        summary,
        root,
        None,
        None,
        None,
        &SecurityBoundarySection::default(),
    )
    .expect("failed to build review");
    let output = render_json(&report, None).expect("failed to render review json");

    serde_json::from_str(&output).expect("expected JSON output")
}

fn changed_file(path: &str) -> ChangedFile {
    ChangedFile {
        path: PathBuf::from(path),
        status: ChangeStatus::Modified,
        ranges: Vec::new(),
    }
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
