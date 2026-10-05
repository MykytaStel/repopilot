use super::super::scan_resolved_changed_with_config;
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::scan::config::ScanConfig;
use std::fs;
use std::path::Path;

#[test]
fn changed_context_graph_is_stable_between_cold_and_cached_scans() {
    let temp = tempfile::tempdir().expect("temporary repository");
    let root = temp.path();
    write(root, "src/lib.rs", "pub mod consumer;\npub mod target;\n");
    write(root, "src/consumer.rs", "use crate::target;\n");
    write(root, "src/target.rs", "pub fn target() {}\n");
    write(root, "coverage/ignored.rs", "use crate::target;\n");

    let changed = vec![ChangedFile {
        path: "coverage/ignored.rs".into(),
        status: ChangeStatus::Modified,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }];
    let config = ScanConfig {
        instability_hub_min_fan_in: 3,
        instability_hub_min_instability_pct: 0,
        ..ScanConfig::default()
    };
    let cold = scan_resolved_changed_with_config(
        root,
        &config,
        root.to_path_buf(),
        changed.clone(),
        Some("HEAD"),
    )
    .expect("cold changed scan");
    let cached =
        scan_resolved_changed_with_config(root, &config, root.to_path_buf(), changed, Some("HEAD"))
            .expect("cached changed scan");
    // The second scan must use the cache, or this compares two cold graphs.
    assert_eq!(
        cached
            .artifacts
            .context_graph_cache
            .as_ref()
            .map(|cache| cache.status.as_str()),
        Some("hit"),
        "{:?}",
        cached.artifacts.context_graph_cache
    );

    let cold_graph = cold
        .artifacts
        .coupling_graph
        .as_ref()
        .expect("cold scan context graph");
    let cached_graph = cached
        .artifacts
        .coupling_graph
        .as_ref()
        .expect("cached scan context graph");

    assert_eq!(cold_graph, cached_graph);
    let cold_hubs = changed_file_hub_findings(&cold);
    let cached_hubs = changed_file_hub_findings(&cached);
    assert_eq!(
        cold_hubs,
        cached_hubs,
        "cold cache={:?}, cached cache={:?}, cached edges={:?}",
        cold.artifacts.context_graph_cache,
        cached.artifacts.context_graph_cache,
        cached_graph.edges
    );
    assert_eq!(cold_hubs.len(), 1);
}

#[test]
fn oversized_modified_file_is_removed_from_cold_and_cached_context_graphs() {
    let temp = tempfile::tempdir().expect("temporary repository");
    let root = temp.path();
    write(root, "src/lib.rs", "pub mod target;\n");
    write(root, "src/target.rs", "pub fn target() {}\n");
    write(
        root,
        "src/large.rs",
        "use crate::target;\npub fn before() {}\n",
    );
    let changed = vec![ChangedFile {
        path: "src/large.rs".into(),
        status: ChangeStatus::Modified,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }];
    let config = ScanConfig {
        max_file_bytes: 64,
        ..ScanConfig::default()
    };

    scan_resolved_changed_with_config(
        root,
        &config,
        root.to_path_buf(),
        changed.clone(),
        Some("HEAD"),
    )
    .expect("seed repository context cache");
    write(root, "src/large.rs", &format!("{}\n", "x".repeat(128)));

    let cached = scan_resolved_changed_with_config(
        root,
        &config,
        root.to_path_buf(),
        changed.clone(),
        Some("HEAD"),
    )
    .expect("cached scan after file exceeds size limit");
    fs::remove_file(crate::graph::context::context_graph_cache_path(root))
        .expect("clear context cache to force cold scan");
    let cold =
        scan_resolved_changed_with_config(root, &config, root.to_path_buf(), changed, Some("HEAD"))
            .expect("cold scan after file exceeds size limit");

    let cached_graph = cached
        .artifacts
        .coupling_graph
        .as_ref()
        .expect("cached context graph");
    let cold_graph = cold
        .artifacts
        .coupling_graph
        .as_ref()
        .expect("cold context graph");
    assert_eq!(
        cold_graph,
        cached_graph,
        "cold cache={:?}, cached cache={:?}, cached nodes={:?}",
        cold.artifacts.context_graph_cache,
        cached.artifacts.context_graph_cache,
        cached_graph.nodes,
    );
    assert!(
        !cached_graph
            .edges
            .get(Path::new("src/large.rs"))
            .is_some_and(|targets| targets.contains(Path::new("src/target.rs")))
    );
}

/// A test file is skipped by the default scan policy, so it is never a graph
/// node, cold or cached. Changing only tests must not rebuild the repository
/// context on every review (next.js: ~5 s per agent stop).
#[test]
fn a_change_to_a_policy_skipped_test_file_uses_the_context_cache() {
    let temp = tempfile::tempdir().expect("temporary repository");
    let root = temp.path();
    write(root, "src/lib.rs", "pub mod target;\n");
    write(root, "src/target.rs", "pub fn target() {}\n");
    write(root, "tests/target.rs", "#[test]\nfn works() {}\n");
    let changed = vec![ChangedFile {
        path: "tests/target.rs".into(),
        status: ChangeStatus::Modified,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }];
    let config = ScanConfig::default();
    let cold = scan_resolved_changed_with_config(
        root,
        &config,
        root.to_path_buf(),
        changed.clone(),
        Some("HEAD"),
    )
    .expect("cold changed scan");
    write(root, "tests/target.rs", "#[test]\nfn still_works() {}\n");
    let cached =
        scan_resolved_changed_with_config(root, &config, root.to_path_buf(), changed, Some("HEAD"))
            .expect("cached changed scan");

    assert_eq!(
        cached
            .artifacts
            .context_graph_cache
            .as_ref()
            .map(|cache| cache.status.as_str()),
        Some("hit"),
        "{:?}",
        cached.artifacts.context_graph_cache
    );
    assert_eq!(
        cold.artifacts.coupling_graph, cached.artifacts.coupling_graph,
        "a skipped file leaves the graph unchanged"
    );
}

fn changed_file_hub_findings(
    summary: &crate::scan::types::ScanSummary,
) -> Vec<crate::findings::types::Finding> {
    summary
        .artifacts
        .findings
        .iter()
        .filter(|finding| {
            finding.rule_id == "architecture.high-instability-hub"
                && finding.evidence[0].path == Path::new("src/target.rs")
        })
        .cloned()
        .collect()
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent directory")).expect("create parent");
    fs::write(path, content).expect("write source file");
}
