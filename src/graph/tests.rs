use super::*;
use crate::scan::facts::{FileFacts, ScanFacts};

fn file(path: &str, language: &str, imports: &[&str]) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: Some(language.to_string()),
        non_empty_lines: 0,
        branch_count: 0,
        imports: imports.iter().map(|value| (*value).to_string()).collect(),
        content: None,
        has_inline_tests: false,
        in_executable_package: false,
        deferred_imports: Vec::new(),
        content_markers: Default::default(),
    }
}

fn py(path: &str, imports: &[&str]) -> FileFacts {
    file(path, "Python", imports)
}

#[test]
fn records_unresolved_workspace_import_but_not_third_party() {
    let facts = ScanFacts {
        root_path: PathBuf::from("/repo"),
        files: vec![file(
            "apps/ml/app/api.py",
            "Python",
            &["app.enrichment.contract", "numpy", "fastapi"],
        )],
        ..ScanFacts::default()
    };

    let (_graph, resolution) = build_coupling_graph_with_resolution(&facts, Path::new("/repo"));

    assert_eq!(
        resolution.total(),
        1,
        "only the internal workspace import should be recorded"
    );
    assert!(resolution.could_target_stem("contract"));
}

#[test]
fn records_missing_configured_tsconfig_alias_with_local_alias_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(
        root.join("tsconfig.json"),
        r#"{"compilerOptions":{"paths":{"@app/*":["src/*"]}}}"#,
    )
    .unwrap();
    let source = root.join("src/app.ts");
    let facts = ScanFacts {
        root_path: root.to_path_buf(),
        files: vec![FileFacts {
            path: source.clone(),
            language: Some("TypeScript".to_string()),
            imports: vec!["@app/missing".to_string()],
            ..FileFacts::default()
        }],
        ..ScanFacts::default()
    };

    let (_graph, resolution) = build_coupling_graph_with_resolution(&facts, root);
    let evidence = resolution.evidence().next().expect("alias is internal");

    assert_eq!(resolution.total(), 1);
    assert_eq!(evidence.kind, UnresolvedImportKind::LocalAlias);
    assert!(matches!(
        &evidence.proof,
        UnresolvedImportProof::DefinitiveLocalCandidates(candidates)
            if candidates.contains(&root.join("src/missing.ts"))
    ));
}

#[test]
fn jvm_unresolved_imports_require_a_matching_local_package() {
    let facts = ScanFacts {
        root_path: PathBuf::from("/repo"),
        files: vec![
            file(
                "app/src/main/java/com/acme/App.java",
                "Java",
                &["com.google.gson.Gson", "com.acme.service.Missing"],
            ),
            file(
                "app/src/main/java/com/acme/service/Existing.java",
                "Java",
                &[],
            ),
        ],
        ..ScanFacts::default()
    };

    let (_graph, resolution) = build_coupling_graph_with_resolution(&facts, Path::new("/repo"));

    assert_eq!(
        resolution.total(),
        1,
        "only the import whose package exists in this repository is internal"
    );
    assert!(resolution.could_target_stem("Missing"));
    assert!(!resolution.could_target_stem("Gson"));
}

#[test]
fn same_package_java_and_kotlin_types_create_local_graph_edges() {
    let facts = ScanFacts {
        root_path: PathBuf::from("/repo"),
        files: vec![
            FileFacts {
                path: PathBuf::from("app/src/main/java/com/acme/App.java"),
                language: Some("Java".to_string()),
                content: Some("package com.acme;\nclass App { Engine engine; }\n".to_string()),
                imports: vec!["com.acme.Engine".to_string()],
                ..FileFacts::default()
            },
            FileFacts {
                path: PathBuf::from("app/src/main/java/com/acme/Engine.java"),
                language: Some("Java".to_string()),
                content: Some("package com.acme;\nclass Engine {}\n".to_string()),
                ..FileFacts::default()
            },
            FileFacts {
                path: PathBuf::from("feature/src/main/kotlin/com/feature/View.kt"),
                language: Some("Kotlin".to_string()),
                content: Some("package com.feature\nclass View(val engine: Engine)\n".to_string()),
                imports: vec!["com.feature.Engine".to_string()],
                ..FileFacts::default()
            },
            FileFacts {
                path: PathBuf::from("feature/src/main/kotlin/com/feature/Engine.kt"),
                language: Some("Kotlin".to_string()),
                content: Some("package com.feature\nclass Engine\n".to_string()),
                ..FileFacts::default()
            },
        ],
        ..ScanFacts::default()
    };

    let (graph, _) = build_coupling_graph_with_resolution(&facts, Path::new("/repo"));

    assert!(
        graph.edges[&PathBuf::from("app/src/main/java/com/acme/App.java")]
            .contains(&PathBuf::from("app/src/main/java/com/acme/Engine.java"))
    );
    assert!(
        graph.edges[&PathBuf::from("feature/src/main/kotlin/com/feature/View.kt")].contains(
            &PathBuf::from("feature/src/main/kotlin/com/feature/Engine.kt")
        )
    );
}

fn graph_from_edges(edges: &[(&str, &str)]) -> CouplingGraph {
    let mut edge_map: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut nodes: BTreeSet<PathBuf> = BTreeSet::new();

    for (src, dst) in edges {
        let src = PathBuf::from(src);
        let dst = PathBuf::from(dst);
        nodes.insert(src.clone());
        nodes.insert(dst.clone());
        edge_map.entry(src).or_default().insert(dst);
    }

    CouplingGraph {
        deferred_edges: Default::default(),
        edges: edge_map,
        nodes,
    }
}

#[test]
fn detects_simple_two_node_cycle() {
    let graph = graph_from_edges(&[("a.rs", "b.rs"), ("b.rs", "a.rs")]);
    let cycles = detect_cycles(&graph);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].len(), 2);
}

#[test]
fn detects_three_node_cycle() {
    let graph = graph_from_edges(&[("a.rs", "b.rs"), ("b.rs", "c.rs"), ("c.rs", "a.rs")]);
    let cycles = detect_cycles(&graph);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].len(), 3);
}

#[test]
fn deferred_back_edge_does_not_form_a_cycle() {
    let mut graph = graph_from_edges(&[("a.py", "b.py"), ("b.py", "a.py")]);
    graph
        .deferred_edges
        .entry(PathBuf::from("b.py"))
        .or_default()
        .insert(PathBuf::from("a.py"));

    assert!(detect_cycles(&graph).is_empty());

    let cycle_graph = without_deferred_edges(&graph);
    assert!(cycle_graph.deferred_edges.is_empty());
    assert!(detect_cycles(&cycle_graph).is_empty());
    assert!(graph.edges[&PathBuf::from("b.py")].contains(&PathBuf::from("a.py")));
}

#[test]
fn no_cycle_in_dag() {
    let graph = graph_from_edges(&[("a.rs", "b.rs"), ("b.rs", "c.rs"), ("a.rs", "c.rs")]);
    assert!(detect_cycles(&graph).is_empty());
}

#[test]
fn rust_module_containment_edges_can_be_removed_before_cycle_detection() {
    let graph = graph_from_edges(&[
        ("src/lib.rs", "src/graph/mod.rs"),
        ("src/graph/mod.rs", "src/graph/context.rs"),
        ("src/graph/context.rs", "src/graph/mod.rs"),
        ("src/a.rs", "src/b.rs"),
        ("src/b.rs", "src/a.rs"),
    ]);

    let filtered = without_rust_module_containment_edges(&graph);
    let cycles = detect_cycles(&filtered);

    assert_eq!(
        cycles,
        vec![vec![PathBuf::from("src/a.rs"), PathBuf::from("src/b.rs")]]
    );
}

#[test]
fn eager_resolved_edge_wins_over_deferred_submodule_alias() {
    let mut source = py("app/views.py", &["app.models", "app.models.thing"]);
    source.deferred_imports = vec!["app.models.thing".to_string()];
    let facts = ScanFacts {
        root_path: PathBuf::from(""),
        files: vec![source, py("app/models.py", &[])],
        ..ScanFacts::default()
    };

    let graph = build_coupling_graph(&facts, Path::new(""));
    let source = PathBuf::from("app/views.py");
    let target = PathBuf::from("app/models.py");

    assert!(graph.edges[&source].contains(&target));
    assert!(
        !graph
            .deferred_edges
            .get(&source)
            .is_some_and(|targets| targets.contains(&target)),
        "a deferred imported symbol that resolves to an eager module edge must not strip that edge"
    );
}

#[test]
fn disconnected_graph_no_cycle() {
    let graph = graph_from_edges(&[("a.rs", "b.rs"), ("c.rs", "d.rs")]);
    assert!(detect_cycles(&graph).is_empty());
}

#[test]
fn compute_metrics_fan_in_and_fan_out() {
    let graph = graph_from_edges(&[("a.rs", "b.rs"), ("a.rs", "c.rs"), ("b.rs", "c.rs")]);

    let metrics = compute_metrics(&graph);
    let find = |name: &str| -> Option<(usize, usize)> {
        metrics
            .iter()
            .find(|m| m.path == std::path::Path::new(name))
            .map(|m| (m.fan_in, m.fan_out))
    };

    let (a_in, a_out) = find("a.rs").expect("a.rs metrics should exist");
    let (b_in, b_out) = find("b.rs").expect("b.rs metrics should exist");
    let (c_in, c_out) = find("c.rs").expect("c.rs metrics should exist");

    assert_eq!(a_out, 2);
    assert_eq!(a_in, 0);
    assert_eq!(b_out, 1);
    assert_eq!(b_in, 1);
    assert_eq!(c_out, 0);
    assert_eq!(c_in, 2);
}
