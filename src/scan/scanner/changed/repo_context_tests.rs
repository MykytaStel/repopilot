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

/// The cached repository context must give audits the same file facts as a
/// cold scan. It used to read `has_inline_tests` from `is_test`, so a Rust
/// module with inline tests became `source-without-test` on every cached run.
#[test]
fn cold_and_cached_changed_scans_report_the_same_findings() {
    let temp = tempfile::tempdir().expect("temporary repository");
    let root = temp.path();
    write(root, "src/lib.rs", "pub mod parser;\npub mod render;\n");
    write(
        root,
        "src/parser.rs",
        "pub fn parse(input: &str) -> usize {\n    input.len()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn parses() {\n        assert_eq!(super::parse(\"ab\"), 2);\n    }\n}\n",
    );
    write(
        root,
        "src/render.rs",
        "pub fn render() -> String {\n    String::new()\n}\n",
    );
    let changed = vec![ChangedFile {
        path: "src/render.rs".into(),
        status: ChangeStatus::Modified,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }];
    let config = ScanConfig::default();
    let scan = || {
        scan_resolved_changed_with_config(
            root,
            &config,
            root.to_path_buf(),
            changed.clone(),
            Some("HEAD"),
        )
        .expect("changed scan")
    };
    let cold = scan();
    let cached = scan();
    assert_eq!(
        cached
            .artifacts
            .context_graph_cache
            .as_ref()
            .map(|cache| cache.status.as_str()),
        Some("hit")
    );
    assert_eq!(cold.artifacts.findings, cached.artifacts.findings);
    assert_eq!(
        cold.artifacts.coupling_graph,
        cached.artifacts.coupling_graph
    );
    assert!(
        !finding_keys(&cached).contains(&(
            "testing.source-without-test".to_string(),
            "src/parser.rs".to_string()
        )),
        "a module with inline tests has tests"
    );
}

fn finding_keys(summary: &crate::scan::types::ScanSummary) -> Vec<(String, String)> {
    let mut keys = summary
        .artifacts
        .findings
        .iter()
        .map(|finding| {
            let path = finding
                .evidence
                .first()
                .map(|evidence| {
                    evidence
                        .path
                        .strip_prefix(&summary.root_path)
                        .unwrap_or(&evidence.path)
                        .to_string_lossy()
                        .replace('\\', "/")
                })
                .unwrap_or_default();
            (finding.rule_id.clone(), path)
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys
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

fn git(root: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("temporary repository");
    let root = temp.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "RepoPilot Tests"]);
    git(root, &["config", "user.email", "tests@example.invalid"]);
    write(root, ".gitignore", ".repopilot/\n");
    write(root, "src/a.ts", "export const value = 1;\n");
    write(root, "src/b.ts", "export const value = 2;\n");
    write(root, "src/consumer.ts", "import { value } from './missing';\n");
    write(root, "other/control.ts", "export const control = 1;\n");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "base"]);
    temp
}

fn working_scan(path: &Path) -> crate::scan::types::ScanSummary {
    super::super::scan_changed_with_config(path, &ScanConfig::default(), None)
        .expect("working tree changed scan")
}

fn has_edge(summary: &crate::scan::types::ScanSummary, from: &str, to: &str) -> bool {
    summary
        .artifacts
        .coupling_graph
        .as_ref()
        .expect("context graph")
        .edges
        .get(Path::new(from))
        .is_some_and(|targets| targets.contains(Path::new(to)))
}

fn assert_cold_parity(root: &Path, scope: &Path, warm: &crate::scan::types::ScanSummary) {
    fs::remove_file(crate::graph::context::context_graph_cache_path(root))
        .expect("remove context cache");
    let cold = working_scan(scope);
    assert_eq!(warm.artifacts.coupling_graph, cold.artifacts.coupling_graph);
    assert_eq!(warm.artifacts.findings, cold.artifacts.findings);
}

#[test]
fn reverted_dirty_import_cannot_survive_in_cached_context() {
    let temp = git_fixture();
    let root = temp.path();
    write(root, "src/a.ts", "import { value } from './b';\n");
    write(root, "other/control.ts", "export const control = 2;\n");
    let dirty = working_scan(root);
    assert!(has_edge(&dirty, "src/a.ts", "src/b.ts"));
    git(root, &["checkout", "--", "src/a.ts"]);
    let restored = working_scan(root);
    assert!(!has_edge(&restored, "src/a.ts", "src/b.ts"));
    assert_cold_parity(root, root, &restored);
}

#[test]
fn untracked_target_rewires_unchanged_importer_and_removal_clears_it() {
    let temp = git_fixture();
    let root = temp.path();
    write(root, "other/control.ts", "export const control = 2;\n");
    working_scan(root);
    write(root, "src/missing.ts", "export const value = 3;\n");
    let added = working_scan(root);
    assert!(has_edge(&added, "src/consumer.ts", "src/missing.ts"));
    assert_cold_parity(root, root, &added);
    fs::remove_file(root.join("src/missing.ts")).unwrap();
    let removed = working_scan(root);
    assert!(!has_edge(&removed, "src/consumer.ts", "src/missing.ts"));
    assert_cold_parity(root, root, &removed);
}

#[test]
fn scoped_review_cannot_reuse_stale_outside_scope_imports() {
    let temp = git_fixture();
    let root = temp.path();
    write(root, "other/control.ts", "export const control = 2;\n");
    let scope = root.join("other");
    working_scan(&scope);
    write(root, "src/a.ts", "import { value } from './b';\n");
    let updated = working_scan(&scope);
    assert!(has_edge(&updated, "src/a.ts", "src/b.ts"));
    assert_cold_parity(root, &scope, &updated);
}

#[test]
fn same_git_dirty_scope_still_reuses_context() {
    let temp = git_fixture();
    let root = temp.path();
    write(root, "src/a.ts", "import { value } from './b';\n");
    let first = working_scan(root);
    let second = working_scan(root);
    assert_eq!(
        second
            .artifacts
            .context_graph_cache
            .as_ref()
            .map(|cache| cache.status.as_str()),
        Some("hit")
    );
    assert_eq!(first.artifacts.coupling_graph, second.artifacts.coupling_graph);
    assert_eq!(first.artifacts.findings, second.artifacts.findings);
}

#[test]
fn staged_rename_cannot_leave_old_target_in_cached_context() {
    let temp = git_fixture();
    let root = temp.path();
    write(root, "src/a.ts", "import { value } from './b';\n");
    let before = working_scan(root);
    assert!(has_edge(&before, "src/a.ts", "src/b.ts"));
    git(root, &["mv", "src/b.ts", "src/renamed.ts"]);
    let renamed = working_scan(root);
    assert!(!has_edge(&renamed, "src/a.ts", "src/b.ts"));
    let graph = renamed.artifacts.coupling_graph.as_ref().unwrap();
    assert!(!graph.nodes.contains(Path::new("src/b.ts")));
    assert!(graph.nodes.contains(Path::new("src/renamed.ts")));
    assert_cold_parity(root, root, &renamed);
}

#[test]
fn dirty_manifest_requires_a_graph_wide_rebuild() {
    let temp = git_fixture();
    let root = temp.path();
    write(root, "package.json", r#"{"name":"fixture","version":"1.0.0"}"#);
    working_scan(root);
    let second = working_scan(root);
    let cache = second.artifacts.context_graph_cache.as_ref().unwrap();
    assert_eq!(cache.status, "miss");
    assert!(cache.reason.contains("dirty-scope-not-patchable"));
}
