use super::*;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::graph::build_coupling_graph;
use crate::risk::{RiskAssessment, priority_for_score};

#[test]
fn summary_caps_cycles_and_marks_truncation() {
    let mut nodes = Vec::new();
    let mut edges = BTreeMap::new();
    for index in 0..(MAX_CONTEXT_GRAPH_CYCLES + 3) {
        let left = PathBuf::from(format!("src/cycle_{index}_a.rs"));
        let right = PathBuf::from(format!("src/cycle_{index}_b.rs"));
        nodes.push(node(&left));
        nodes.push(node(&right));
        edges
            .entry(left.clone())
            .or_insert_with(BTreeSet::new)
            .insert(right.clone());
        edges
            .entry(right)
            .or_insert_with(BTreeSet::new)
            .insert(left);
    }

    let graph = RepoContextGraph {
        root_path: PathBuf::from("."),
        nodes,
        edges,
        deferred_edges: BTreeMap::new(),
        detected_frameworks: Vec::new(),
        framework_projects: Vec::new(),
        react_native: None,
    };

    let summary = summarize_context_graph(&graph, &[], &[]);

    assert_eq!(summary.cycles.len(), MAX_CONTEXT_GRAPH_CYCLES);
    assert!(summary.truncated.iter().any(|value| value == "cycles"));
}

#[test]
fn summary_caps_risky_clusters_and_marks_truncation() {
    let findings = (0..(MAX_CONTEXT_GRAPH_RISKY_CLUSTERS + 3))
        .map(|index| {
            finding(
                &format!("test.rule-{index}"),
                &format!("src/area_{index}/file.rs"),
                80,
            )
        })
        .collect::<Vec<_>>();
    let graph = RepoContextGraph {
        root_path: PathBuf::from("."),
        nodes: Vec::new(),
        edges: BTreeMap::new(),
        deferred_edges: BTreeMap::new(),
        detected_frameworks: Vec::new(),
        framework_projects: Vec::new(),
        react_native: None,
    };

    let summary = summarize_context_graph(&graph, &findings, &[]);

    assert_eq!(
        summary.risky_clusters.len(),
        MAX_CONTEXT_GRAPH_RISKY_CLUSTERS
    );
    assert!(
        summary
            .truncated
            .iter()
            .any(|value| value == "risky_clusters")
    );
}

#[test]
fn summary_changed_blast_radius_lists_direct_importers() {
    // a.rs and b.rs both import shared.rs; changing shared.rs reaches both,
    // sourced through the shared graph v2 one-hop dependents.
    let shared = PathBuf::from("src/shared.rs");
    let a = PathBuf::from("src/a.rs");
    let b = PathBuf::from("src/b.rs");
    let mut edges = BTreeMap::new();
    edges.insert(a.clone(), BTreeSet::from([shared.clone()]));
    edges.insert(b.clone(), BTreeSet::from([shared.clone()]));

    let graph = RepoContextGraph {
        root_path: PathBuf::from("."),
        nodes: vec![node(&a), node(&b), node(&shared)],
        edges,
        deferred_edges: BTreeMap::new(),
        detected_frameworks: Vec::new(),
        framework_projects: Vec::new(),
        react_native: None,
    };
    let changed = vec![ChangedFile {
        path: shared.clone(),
        status: ChangeStatus::Modified,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }];

    let summary = summarize_context_graph(&graph, &[], &changed);

    assert_eq!(summary.changed_blast_radius, vec![a, b]);
}

#[test]
fn changed_graph_patch_matches_full_rebuild_for_add_modify_delete() {
    let root = PathBuf::from("/repo");
    let initial_facts = scan_facts(
        &root,
        vec![
            file_fact("src/a.ts", &["./b"]),
            file_fact("src/b.ts", &[]),
            file_fact("src/old.ts", &["./b"]),
        ],
    );
    let initial_coupling = build_coupling_graph(&initial_facts, &root);
    let mut patched_graph =
        RepoContextGraph::from_scan_facts(&initial_facts, &root, initial_coupling);

    let changed_files = vec![
        ChangedFile {
            path: PathBuf::from("src/a.ts"),
            status: ChangeStatus::Modified,
            ranges: Vec::new(),
            hunks: Vec::new(),
        },
        ChangedFile {
            path: PathBuf::from("src/new.ts"),
            status: ChangeStatus::Added,
            ranges: Vec::new(),
            hunks: Vec::new(),
        },
        ChangedFile {
            path: PathBuf::from("src/old.ts"),
            status: ChangeStatus::Deleted,
            ranges: Vec::new(),
            hunks: Vec::new(),
        },
    ];
    let patch_files = vec![
        file_fact("src/a.ts", &["./new"]),
        file_fact("src/new.ts", &["./b"]),
    ];

    patched_graph.apply_changed_facts(&root, &changed_files, &patch_files);

    let expected_facts = scan_facts(
        &root,
        vec![
            file_fact("src/a.ts", &["./new"]),
            file_fact("src/b.ts", &[]),
            file_fact("src/new.ts", &["./b"]),
        ],
    );
    let expected_coupling = build_coupling_graph(&expected_facts, &root);
    let expected_graph =
        RepoContextGraph::from_scan_facts(&expected_facts, &root, expected_coupling);

    assert_eq!(patched_graph.nodes, expected_graph.nodes);
    assert_eq!(patched_graph.edges, expected_graph.edges);
    assert_eq!(patched_graph.deferred_edges, expected_graph.deferred_edges);
}

#[test]
fn added_target_rechecks_unchanged_unresolved_importer() {
    let root = PathBuf::from("/repo");
    let initial_facts = scan_facts(&root, vec![file_fact("src/a.ts", &["./new"])]);
    let initial_coupling = build_coupling_graph(&initial_facts, &root);
    let mut patched_graph =
        RepoContextGraph::from_scan_facts(&initial_facts, &root, initial_coupling);
    assert!(
        patched_graph
            .edges
            .get(Path::new("src/a.ts"))
            .is_some_and(BTreeSet::is_empty),
        "unresolved import should not have an edge before target exists"
    );

    let changed_files = vec![ChangedFile {
        path: PathBuf::from("src/new.ts"),
        status: ChangeStatus::Added,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }];
    let patch_files = vec![file_fact("src/new.ts", &[])];

    patched_graph.apply_changed_facts(&root, &changed_files, &patch_files);

    let expected_facts = scan_facts(
        &root,
        vec![
            file_fact("src/a.ts", &["./new"]),
            file_fact("src/new.ts", &[]),
        ],
    );
    let expected_coupling = build_coupling_graph(&expected_facts, &root);
    let expected_graph =
        RepoContextGraph::from_scan_facts(&expected_facts, &root, expected_coupling);

    assert_eq!(patched_graph.edges, expected_graph.edges);
    assert_eq!(
        patched_graph.edges.get(Path::new("src/a.ts")),
        Some(&BTreeSet::from([PathBuf::from("src/new.ts")]))
    );
}

#[test]
fn deleted_preferred_target_rechecks_unchanged_importer_for_fallback() {
    let root = PathBuf::from("/repo");
    let initial_facts = scan_facts(
        &root,
        vec![
            file_fact("src/a.ts", &["./foo"]),
            file_fact("src/foo.ts", &[]),
            file_fact("src/foo/index.ts", &[]),
        ],
    );
    let initial_coupling = build_coupling_graph(&initial_facts, &root);
    let mut patched_graph =
        RepoContextGraph::from_scan_facts(&initial_facts, &root, initial_coupling);
    assert_eq!(
        patched_graph.edges.get(Path::new("src/a.ts")),
        Some(&BTreeSet::from([PathBuf::from("src/foo.ts")]))
    );

    let changed_files = vec![ChangedFile {
        path: PathBuf::from("src/foo.ts"),
        status: ChangeStatus::Deleted,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }];

    patched_graph.apply_changed_facts(&root, &changed_files, &[]);

    let expected_facts = scan_facts(
        &root,
        vec![
            file_fact("src/a.ts", &["./foo"]),
            file_fact("src/foo/index.ts", &[]),
        ],
    );
    let expected_coupling = build_coupling_graph(&expected_facts, &root);
    let expected_graph =
        RepoContextGraph::from_scan_facts(&expected_facts, &root, expected_coupling);

    assert_eq!(patched_graph.edges, expected_graph.edges);
    assert_eq!(
        patched_graph.edges.get(Path::new("src/a.ts")),
        Some(&BTreeSet::from([PathBuf::from("src/foo/index.ts")]))
    );
}

fn scan_facts(root: &Path, files: Vec<FileFacts>) -> ScanFacts {
    ScanFacts {
        root_path: root.to_path_buf(),
        files_discovered: files.len(),
        files_analyzed: files.len(),
        files,
        ..ScanFacts::default()
    }
}

fn file_fact(path: &str, imports: &[&str]) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: Some("TypeScript".to_string()),
        non_empty_lines: 1,
        branch_count: 0,
        imports: imports.iter().map(|value| (*value).to_string()).collect(),
        deferred_imports: Vec::new(),
        content: None,
        has_inline_tests: false,
        in_executable_package: false,
    }
}

fn node(path: &Path) -> RepoContextNode {
    RepoContextNode {
        path: path.to_path_buf(),
        language: Some("Rust".to_string()),
        roles: Vec::new(),
        frameworks: Vec::new(),
        runtimes: Vec::new(),
        paradigms: Vec::new(),
        workspace_package: None,
        non_empty_lines: 1,
        imports: Vec::new(),
        deferred_imports: Vec::new(),
        is_test: false,
        is_generated: false,
        is_config: false,
    }
}

fn finding(rule_id: &str, path: &str, score: u8) -> Finding {
    Finding {
        id: String::new(),
        rule_id: rule_id.to_string(),
        title: String::new(),
        description: String::new(),
        recommendation: String::new(),
        category: FindingCategory::Architecture,
        severity: Severity::High,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from(path),
            line_start: 1,
            line_end: None,
            snippet: String::new(),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: RiskAssessment {
            score,
            priority: priority_for_score(score),
            ..RiskAssessment::default()
        },
    }
}
