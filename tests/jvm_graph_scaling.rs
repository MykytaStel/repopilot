//! JVM assembly parity is semantic, not a host-dependent stopwatch assertion.
use repopilot::graph::context::RepoContextGraph;
use repopilot::graph::v2::graph_snapshot_from_scan;
use repopilot::graph::{build_coupling_graph_with_resolution, resolve_import};
use repopilot::review::diff::{ChangeStatus, ChangedFile};
use repopilot::scan::facts::{FileFacts, ScanFacts};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

fn file(path: PathBuf, imports: Vec<String>) -> FileFacts {
    FileFacts {
        path,
        imports,
        ..FileFacts::default()
    }
}

fn workload() -> ScanFacts {
    let mut files = Vec::new();
    for i in 0..120 {
        let next = (i + 1) % 120;
        files.push(file(
            PathBuf::from(format!(
                "/repo/m{i}/src/main/kotlin/com/example/p{i}/Type{i}.kt"
            )),
            vec![
                format!("com.example.p{next}.Type{next}"),
                format!("com.example.p{next}.Type{next}.Companion.VALUE"),
                format!("com.example.p{next}.Missing"),
                "com.example.Ambiguous".into(),
            ],
        ));
    }
    for (i, file) in files.iter_mut().enumerate() {
        let deferred = (i + 2) % 120;
        file.deferred_imports = vec![format!("com.example.p{deferred}.Type{deferred}")];
    }
    for variant in ["demo", "prod"] {
        files.push(file(
            PathBuf::from(format!("/repo/{variant}/com/example/Ambiguous.kt")),
            Vec::new(),
        ));
    }
    ScanFacts {
        root_path: PathBuf::from("/repo"),
        files,
        ..ScanFacts::default()
    }
}

#[test]
fn jvm_complete_edges_and_unresolved_evidence_survive_order_and_warm_rebuilds() {
    let mut facts = workload();
    let known = facts
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<HashSet<_>>();
    let expected = facts
        .files
        .iter()
        .map(|file| {
            let targets = file
                .imports
                .iter()
                .filter_map(|raw| resolve_import(raw, &file.path, &facts.root_path, &known))
                .collect::<BTreeSet<_>>();
            (file.path.clone(), targets)
        })
        .collect::<BTreeMap<_, _>>();
    let cold = build_coupling_graph_with_resolution(&facts, &facts.root_path);
    assert_eq!(cold.0.edges, expected);
    for (i, file) in facts.files.iter().take(120).enumerate() {
        let deferred = &facts.files[(i + 2) % 120].path;
        assert_eq!(
            cold.0.deferred_edges.get(&file.path),
            Some(&BTreeSet::from([deferred.clone()]))
        );
    }
    let cold_snapshot = graph_snapshot_from_scan(&facts);
    assert_eq!(cold.1.unresolved_internal_by_source.len(), 120);
    assert!(
        cold.1
            .unresolved_internal_by_source
            .values()
            .all(|evidence| evidence.len() == 2)
    );
    for _ in 0..2 {
        facts.files.reverse();
        let warm = build_coupling_graph_with_resolution(&facts, &facts.root_path);
        assert_eq!(warm.0.edges, cold.0.edges);
        assert_eq!(warm.0.deferred_edges, cold.0.deferred_edges);
        assert_eq!(warm.1, cold.1);
        assert_eq!(graph_snapshot_from_scan(&facts), cold_snapshot);
    }
}

#[test]
fn jvm_serialized_context_patch_rebuilds_inventory_after_ambiguity_disappears() {
    let mut facts = workload();
    let coupling = build_coupling_graph_with_resolution(&facts, &facts.root_path).0;
    let cold = RepoContextGraph::from_scan_facts(&facts, &facts.root_path, coupling);
    let mut warm: RepoContextGraph =
        serde_json::from_slice(&serde_json::to_vec(&cold).unwrap()).unwrap();
    assert_eq!(warm, cold);
    let deleted = PathBuf::from("demo/com/example/Ambiguous.kt");
    warm.apply_changed_facts(
        Path::new("/repo"),
        &[ChangedFile {
            path: deleted,
            status: ChangeStatus::Deleted,
            ranges: Vec::new(),
            hunks: Vec::new(),
        }],
        &[],
    );
    facts
        .files
        .retain(|file| !file.path.starts_with("/repo/demo"));
    let expected = RepoContextGraph::from_scan_facts(
        &facts,
        &facts.root_path,
        build_coupling_graph_with_resolution(&facts, &facts.root_path).0,
    );
    assert_eq!(warm.edges, expected.edges);
    assert_eq!(warm.deferred_edges, expected.deferred_edges);
    assert!(
        warm.edges
            .values()
            .filter(|targets| targets.contains(Path::new("prod/com/example/Ambiguous.kt")))
            .count()
            == 120
    );
}
