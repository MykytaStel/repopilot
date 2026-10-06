use super::super::scan_resolved_changed_with_config;
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::scan::{config::ScanConfig, types::ScanSummary};
use std::{fs, path::Path};

fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join("src")).unwrap();
    for name in ["one", "two"] {
        fs::write(
            root.join(format!("src/{name}.ts")),
            "export const api = 1;\n",
        )
        .unwrap();
    }
    fs::write(
        root.join("src/consumer.ts"),
        "import { api } from '@api';\nexport const result = api;\n",
    )
    .unwrap();
    temp
}

fn remap(root: &Path, name: &str, target: &str) {
    fs::write(
        root.join(name),
        format!(r#"{{"compilerOptions":{{"paths":{{"@api":["src/{target}"]}}}}}}"#),
    )
    .unwrap();
}

fn scan(root: &Path) -> ScanSummary {
    scan_resolved_changed_with_config(
        root,
        &ScanConfig::default(),
        root.to_path_buf(),
        vec![ChangedFile {
            path: "tsconfig.json".into(),
            status: ChangeStatus::Modified,
            ranges: vec![],
            hunks: vec![],
        }],
        Some("HEAD"),
    )
    .unwrap()
}

fn assert_semantics(left: &ScanSummary, right: &ScanSummary) {
    assert_eq!(left.artifacts.findings, right.artifacts.findings);
    assert_eq!(
        left.artifacts.coupling_graph,
        right.artifacts.coupling_graph
    );
}

fn assert_hit(summary: &ScanSummary) {
    assert_eq!(
        summary
            .artifacts
            .context_graph_cache
            .as_ref()
            .unwrap()
            .status,
        "hit"
    );
}

#[test]
fn config_only_alias_remap_matches_cold_context_and_reuses_unchanged_inputs() {
    let temp = fixture();
    let root = temp.path();
    remap(root, "tsconfig.json", "one");
    let before = scan(root);
    remap(root, "tsconfig.json", "two");
    let warm = scan(root);
    let graph = warm.artifacts.coupling_graph.as_ref().unwrap();
    assert_ne!(
        before.artifacts.coupling_graph.as_ref().unwrap().edges,
        graph.edges
    );
    assert!(
        graph
            .edges
            .values()
            .any(|targets| targets.contains(Path::new("src/two.ts")))
    );
    fs::remove_file(crate::graph::context::context_graph_cache_path(root)).unwrap();
    let cold = scan(root);
    assert_semantics(&warm, &cold);
    let reused = scan(root);
    assert_hit(&reused);
    assert_semantics(&cold, &reused);
}

#[test]
fn inactive_config_preserves_cached_edges_and_deletion_falls_back() {
    let temp = fixture();
    let root = temp.path();
    remap(root, "tsconfig.json", "two");
    let preferred = scan(root);
    remap(root, "jsconfig.json", "one");
    let inactive = scan(root);
    assert_hit(&inactive);
    assert_eq!(
        preferred.artifacts.coupling_graph.as_ref().unwrap().edges,
        inactive.artifacts.coupling_graph.as_ref().unwrap().edges
    );
    remap(root, "jsconfig.json", "two");
    assert_hit(&scan(root));
    remap(root, "jsconfig.json", "one");
    fs::remove_file(root.join("tsconfig.json")).unwrap();
    let fallback = scan(root);
    assert_eq!(
        fallback.artifacts.coupling_graph.as_ref().unwrap().edges[Path::new("src/consumer.ts")],
        ["src/one.ts".into()].into_iter().collect()
    );
}

#[test]
fn legacy_context_without_resolver_identity_rebuilds_once() {
    let temp = fixture();
    let root = temp.path();
    remap(root, "tsconfig.json", "two");
    let current = scan(root);
    let cache_path = crate::graph::context::context_graph_cache_path(root);
    let mut legacy: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&cache_path).unwrap()).unwrap();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("resolver_input_fingerprint");
    fs::write(&cache_path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    let upgraded = scan(root);
    assert_ne!(
        upgraded
            .artifacts
            .context_graph_cache
            .as_ref()
            .unwrap()
            .status,
        "hit"
    );
    assert_semantics(&current, &upgraded);
    assert_hit(&scan(root));
}
