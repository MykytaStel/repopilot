use super::*;
use crate::scan::facts::FileFacts;

fn file(path: &str, imports: &[&str]) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: None,
        non_empty_lines: 0,
        branch_count: 0,
        imports: imports.iter().map(|value| (*value).to_string()).collect(),
        content: None,
        has_inline_tests: false,
    }
}

fn scan(files: Vec<FileFacts>) -> ScanFacts {
    ScanFacts {
        root_path: PathBuf::from("/repo"),
        files,
        ..ScanFacts::default()
    }
}

#[test]
fn empty_scan_creates_empty_snapshot() {
    assert_eq!(
        graph_snapshot_from_scan(&ScanFacts::default()),
        GraphSnapshot::default()
    );
}

#[test]
fn file_nodes_use_relative_stable_ids_and_keep_path_metadata() {
    let snapshot = graph_snapshot_from_scan(&scan(vec![file("/repo/src/main.rs", &[])]));

    assert_eq!(snapshot.node_count(), 1);
    assert_eq!(snapshot.nodes[0].id.as_str(), "file:src/main.rs");
    assert_eq!(snapshot.nodes[0].label, "src/main.rs");
    assert_eq!(
        snapshot.nodes[0].path.as_deref(),
        Some(Path::new("/repo/src/main.rs"))
    );
    assert!(!snapshot.nodes[0].id.as_str().contains("/repo"));
}

#[test]
fn relative_import_resolves_common_extension_and_deduplicates_edges() {
    let snapshot = graph_snapshot_from_scan(&scan(vec![
        file("/repo/src/main.ts", &["./util", "./util", "./components"]),
        file("/repo/src/util.ts", &[]),
        file("/repo/src/components/index.ts", &[]),
    ]));

    assert_eq!(snapshot.node_count(), 3);
    assert_eq!(snapshot.edge_count(), 2);
    assert_eq!(snapshot.edges[0].from.as_str(), "file:src/main.ts");
    assert_eq!(
        snapshot.edges[0].to.as_str(),
        "file:src/components/index.ts"
    );
    // A resolved local import is a high-confidence `Imports` edge.
    assert_eq!(snapshot.edges[0].kind, GraphEdgeKind::Imports);
    assert_eq!(snapshot.edges[0].provenance, GraphEdgeProvenance::Import);
    assert_eq!(snapshot.edges[0].confidence, GraphEdgeConfidence::High);
    assert_eq!(snapshot.edges[1].to.as_str(), "file:src/util.ts");
}

#[test]
fn package_and_unresolved_relative_imports_create_external_nodes() {
    let snapshot = graph_snapshot_from_scan(&scan(vec![file(
        "/repo/src/main.ts",
        &["react", "./missing"],
    )]));

    assert_eq!(snapshot.node_count(), 3);
    assert_eq!(snapshot.edge_count(), 2);
    assert!(
        snapshot
            .nodes
            .iter()
            .any(|node| node.id.as_str() == "external:react"
                && node.kind == GraphNodeKind::ExternalDependency
                && node.path.is_none())
    );

    // A bare package import is a medium-confidence external dependency.
    let react = snapshot
        .edges
        .iter()
        .find(|edge| edge.to.as_str() == "external:react")
        .expect("external package edge");
    assert_eq!(react.kind, GraphEdgeKind::DependsOn);
    assert_eq!(react.confidence, GraphEdgeConfidence::Medium);

    // An unresolved relative import is low confidence and also diagnosed.
    let missing = snapshot
        .edges
        .iter()
        .find(|edge| edge.to.as_str() == "external:./missing")
        .expect("unresolved relative import edge");
    assert_eq!(missing.kind, GraphEdgeKind::DependsOn);
    assert_eq!(missing.confidence, GraphEdgeConfidence::Low);
    assert_eq!(snapshot.diagnostic_count(), 1);
    assert_eq!(snapshot.diagnostics[0].code, "graph-v2.unresolved-import");
}

#[test]
fn snapshot_order_is_independent_of_file_and_import_order() {
    let first = graph_snapshot_from_scan(&scan(vec![
        file("/repo/src/b.ts", &["react", "./a"]),
        file("/repo/src/a.ts", &[]),
    ]));
    let second = graph_snapshot_from_scan(&scan(vec![
        file("/repo/src/a.ts", &[]),
        file("/repo/src/b.ts", &["./a", "react"]),
    ]));

    assert_eq!(first, second);
}

#[test]
fn relative_import_resolves_deterministically_via_shared_resolver() {
    // With both `util.js` and `util.ts` present, the shared TypeScript
    // resolver probes extensions in a fixed order and picks `util.ts`, so
    // there is no ambiguity and no diagnostic.
    let snapshot = graph_snapshot_from_scan(&scan(vec![
        file("/repo/src/main.ts", &["./util"]),
        file("/repo/src/util.js", &[]),
        file("/repo/src/util.ts", &[]),
    ]));

    assert_eq!(snapshot.edge_count(), 1);
    assert_eq!(snapshot.diagnostic_count(), 0);
    assert_eq!(snapshot.edges[0].to.as_str(), "file:src/util.ts");
}

#[test]
fn rust_crate_import_resolves_to_local_module() {
    let snapshot = graph_snapshot_from_scan(&scan(vec![
        file("/repo/src/lib.rs", &["crate::foo"]),
        file("/repo/src/foo.rs", &[]),
    ]));

    assert_eq!(snapshot.edge_count(), 1);
    assert_eq!(snapshot.edges[0].from.as_str(), "file:src/lib.rs");
    assert_eq!(snapshot.edges[0].to.as_str(), "file:src/foo.rs");
    // `crate::` now resolves to a real file node instead of `external:`.
    assert!(
        snapshot
            .nodes
            .iter()
            .all(|node| node.kind != GraphNodeKind::ExternalDependency)
    );
}

#[test]
fn rust_super_import_resolves_to_parent_module() {
    let snapshot = graph_snapshot_from_scan(&scan(vec![
        file("/repo/src/app/handler.rs", &["super::config"]),
        file("/repo/src/app/config.rs", &[]),
    ]));

    assert_eq!(snapshot.edge_count(), 1);
    assert_eq!(snapshot.edges[0].from.as_str(), "file:src/app/handler.rs");
    assert_eq!(snapshot.edges[0].to.as_str(), "file:src/app/config.rs");
}

#[test]
fn test_files_link_to_their_subject_with_a_test_of_edge() {
    let snapshot = graph_snapshot_from_scan(&scan(vec![
        file("/repo/src/math.ts", &[]),
        file("/repo/src/math.test.ts", &["./math"]),
    ]));

    // The TestOf edge is emitted alongside the import edge between the pair.
    let test_of = snapshot
        .edges
        .iter()
        .find(|edge| edge.kind == GraphEdgeKind::TestOf)
        .expect("test-of edge");
    assert_eq!(test_of.from.as_str(), "file:src/math.test.ts");
    assert_eq!(test_of.to.as_str(), "file:src/math.ts");
    assert_eq!(test_of.provenance, GraphEdgeProvenance::TestHeuristic);
    assert_eq!(test_of.confidence, GraphEdgeConfidence::High);
}

// ── Workspace package nodes ─────────────────────────────────────────────────
//
// Package detection reads manifests from disk, so these build a real workspace
// on a temp dir and a `ScanFacts` whose file paths live under it.

use std::fs;
use tempfile::TempDir;

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn scan_under(root: &Path, relatives: &[&str]) -> ScanFacts {
    let files = relatives
        .iter()
        .map(|relative| file(root.join(relative).to_string_lossy().as_ref(), &[]))
        .collect();
    ScanFacts {
        root_path: root.to_path_buf(),
        files,
        ..ScanFacts::default()
    }
}

fn package_nodes(snapshot: &GraphSnapshot) -> Vec<&str> {
    snapshot
        .nodes
        .iter()
        .filter(|node| node.kind == GraphNodeKind::Package)
        .map(|node| node.id.as_str())
        .collect()
}

#[test]
fn npm_workspace_creates_package_nodes_and_membership() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "package.json", r#"{ "workspaces": ["packages/*"] }"#);
    write(
        root,
        "packages/web/package.json",
        r#"{ "name": "@acme/web" }"#,
    );
    write(
        root,
        "packages/api/package.json",
        r#"{ "name": "@acme/api" }"#,
    );

    let snapshot = graph_snapshot_from_scan(&scan_under(
        root,
        &["packages/web/src/index.ts", "packages/api/src/index.ts"],
    ));

    assert_eq!(
        package_nodes(&snapshot),
        vec!["package:packages/api", "package:packages/web"]
    );
    // The Package node label carries the manifest name for consumers.
    let web = snapshot
        .nodes
        .iter()
        .find(|node| node.id.as_str() == "package:packages/web")
        .unwrap();
    assert_eq!(web.label, "@acme/web");

    let membership = &snapshot.package_membership;
    assert_eq!(
        membership
            .get(&GraphNodeId::new("file:packages/web/src/index.ts"))
            .unwrap()
            .as_str(),
        "package:packages/web"
    );
    assert_eq!(
        membership
            .get(&GraphNodeId::new("file:packages/api/src/index.ts"))
            .unwrap()
            .as_str(),
        "package:packages/api"
    );
}

#[test]
fn cargo_workspace_creates_package_nodes() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\n",
    );
    write(
        root,
        "crates/core/Cargo.toml",
        "[package]\nname = \"core\"\n",
    );
    write(root, "crates/core/src/lib.rs", "");

    let snapshot = graph_snapshot_from_scan(&scan_under(root, &["crates/core/src/lib.rs"]));

    assert_eq!(package_nodes(&snapshot), vec!["package:crates/core"]);
    assert_eq!(
        snapshot
            .package_membership
            .get(&GraphNodeId::new("file:crates/core/src/lib.rs"))
            .unwrap()
            .as_str(),
        "package:crates/core"
    );
}

#[test]
fn go_work_creates_package_nodes() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "go.work", "use (\n\t./svc-a\n)\n");
    write(root, "svc-a/go.mod", "module example.com/svc-a\n");
    write(root, "svc-a/main.go", "package main\n");

    let snapshot = graph_snapshot_from_scan(&scan_under(root, &["svc-a/main.go"]));

    assert_eq!(package_nodes(&snapshot), vec!["package:svc-a"]);
    assert_eq!(
        snapshot
            .package_membership
            .get(&GraphNodeId::new("file:svc-a/main.go"))
            .unwrap()
            .as_str(),
        "package:svc-a"
    );
}

#[test]
fn non_workspace_root_adds_no_package_nodes_or_membership() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "package.json", r#"{ "name": "solo" }"#);

    let snapshot = graph_snapshot_from_scan(&scan_under(root, &["src/index.ts"]));

    assert!(package_nodes(&snapshot).is_empty());
    assert!(snapshot.package_membership.is_empty());
    // The only node is the file itself — the snapshot is unchanged by package support.
    assert_eq!(snapshot.node_count(), 1);
}
