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
