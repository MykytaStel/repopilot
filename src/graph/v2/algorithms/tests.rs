use super::*;
use crate::graph::v2::{
    GraphDiagnostic, GraphEdge, GraphEdgeConfidence, GraphEdgeKind, GraphEdgeProvenance, GraphNode,
    GraphNodeId, GraphNodeKind, GraphSnapshot,
};

fn id(value: &str) -> GraphNodeId {
    GraphNodeId::new(value)
}

fn node(value: &str) -> GraphNode {
    GraphNode {
        id: id(value),
        kind: GraphNodeKind::File,
        label: value.to_string(),
        path: None,
    }
}

fn edge(from: &str, to: &str) -> GraphEdge {
    GraphEdge {
        from: id(from),
        to: id(to),
        kind: GraphEdgeKind::Imports,
        provenance: GraphEdgeProvenance::Import,
        confidence: GraphEdgeConfidence::High,
    }
}

fn snapshot(nodes: &[&str], edges: &[(&str, &str)]) -> GraphSnapshot {
    GraphSnapshot {
        nodes: nodes.iter().map(|value| node(value)).collect(),
        edges: edges.iter().map(|(from, to)| edge(from, to)).collect(),
        ..Default::default()
    }
}

fn degree<'a>(summary: &'a GraphDegreeSummary, node_id: &str) -> &'a NodeDegree {
    summary
        .nodes
        .iter()
        .find(|degree| degree.node_id.as_str() == node_id)
        .expect("node degree should exist")
}

#[test]
fn empty_and_disconnected_graphs_have_deterministic_degree_summaries() {
    assert!(compute_degrees(&GraphSnapshot::default()).nodes.is_empty());

    let summary = compute_degrees(&snapshot(&["b", "a"], &[]));

    assert_eq!(
        summary
            .nodes
            .iter()
            .map(|degree| degree.node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert!(summary.nodes.iter().all(|node| node.fan_in == 0));
    assert!(summary.nodes.iter().all(|node| node.fan_out == 0));
}

#[test]
fn degrees_count_each_exact_edge_once() {
    let graph = snapshot(&["a", "b"], &[("a", "b"), ("a", "b")]);

    let summary = compute_degrees(&graph);

    assert_eq!(degree(&summary, "a").fan_out, 1);
    assert_eq!(degree(&summary, "a").fan_in, 0);
    assert_eq!(degree(&summary, "b").fan_in, 1);
    assert_eq!(degree(&summary, "b").fan_out, 0);
}

#[test]
fn direct_dependents_lists_one_hop_importers_only() {
    // a -> b -> c and a -> c. c is imported directly by a and b; b only by a;
    // a by nobody. Dependents are one hop, never transitive.
    let graph = snapshot(&["a", "b", "c"], &[("a", "b"), ("b", "c"), ("a", "c")]);
    let dependents = direct_dependents(&graph);
    let importers = |center: &str| {
        dependents[&id(center)]
            .iter()
            .map(|node| node.as_str().to_string())
            .collect::<Vec<_>>()
    };

    assert_eq!(importers("c"), vec!["a", "b"]);
    assert_eq!(importers("b"), vec!["a"]);
    assert!(importers("a").is_empty());
}

#[test]
fn node_degree_instability_matches_coupling_formula() {
    let isolated = NodeDegree {
        node_id: id("a"),
        fan_in: 0,
        fan_out: 0,
    };
    let pure_source = NodeDegree {
        node_id: id("b"),
        fan_in: 0,
        fan_out: 3,
    };
    let mixed = NodeDegree {
        node_id: id("c"),
        fan_in: 1,
        fan_out: 3,
    };

    assert_eq!(isolated.instability().to_bits(), 0.0_f32.to_bits());
    assert_eq!(pure_source.instability().to_bits(), 1.0_f32.to_bits());
    assert_eq!(mixed.instability().to_bits(), 0.75_f32.to_bits());
}

#[test]
fn hub_helpers_apply_rankings_limits_and_zero_filtering() {
    let graph = snapshot(
        &["a", "b", "c", "d", "z"],
        &[("a", "c"), ("a", "d"), ("b", "c"), ("b", "d")],
    );
    let summary = compute_degrees(&graph);

    assert_eq!(
        top_fan_in(&summary, 2)
            .iter()
            .map(|node| node.node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["c", "d"]
    );
    assert_eq!(
        top_fan_out(&summary, 2)
            .iter()
            .map(|node| node.node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert!(top_fan_in(&summary, 0).is_empty());
    assert!(top_fan_out(&summary, 0).is_empty());
    assert!(
        top_fan_in(&summary, usize::MAX)
            .iter()
            .all(|node| node.node_id.as_str() != "z")
    );
}

#[test]
fn acyclic_graph_has_no_cycles() {
    assert!(find_cycles(&snapshot(&["a", "b", "c"], &[("a", "b"), ("b", "c")])).is_empty());
}

#[test]
fn tarjan_groups_directed_cycle_and_self_loop() {
    let graph = snapshot(
        &["a", "b", "c", "d"],
        &[("a", "b"), ("b", "c"), ("c", "a"), ("d", "d"), ("d", "d")],
    );

    let cycles = find_cycles(&graph);

    assert_eq!(
        cycles,
        vec![
            GraphCycle {
                node_ids: vec![id("a"), id("b"), id("c")],
            },
            GraphCycle {
                node_ids: vec![id("d")],
            },
        ]
    );
}

#[test]
fn multiple_cycles_sort_by_size_then_first_node() {
    let graph = snapshot(
        &["a", "b", "c", "d", "e", "f"],
        &[
            ("a", "b"),
            ("b", "a"),
            ("c", "d"),
            ("d", "e"),
            ("e", "c"),
            ("f", "f"),
        ],
    );

    let cycles = find_cycles(&graph);

    assert_eq!(cycles[0].node_ids, vec![id("c"), id("d"), id("e")]);
    assert_eq!(cycles[1].node_ids, vec![id("a"), id("b")]);
    assert_eq!(cycles[2].node_ids, vec![id("f")]);
}

#[test]
fn shortest_cycle_returns_the_minimal_loop_within_a_large_component() {
    // One strongly-connected component: a long ring a->b->c->d->a plus a short
    // back-edge c->a, so the minimal cycle is a->b->c->a.
    let graph = snapshot(
        &["a", "b", "c", "d"],
        &[("a", "b"), ("b", "c"), ("c", "d"), ("d", "a"), ("c", "a")],
    );
    let component = &find_cycles(&graph)[0];

    let minimal = shortest_cycle(&graph, component);

    assert_eq!(minimal, vec![id("a"), id("b"), id("c"), id("a")]);
    assert!(minimal.len() < component.node_ids.len() + 1);
}

#[test]
fn shortest_cycle_handles_two_node_loops_and_self_loops() {
    let two = snapshot(&["a", "b"], &[("a", "b"), ("b", "a")]);
    assert_eq!(
        shortest_cycle(&two, &find_cycles(&two)[0]),
        vec![id("a"), id("b"), id("a")]
    );

    let loop_back = snapshot(&["a"], &[("a", "a")]);
    assert_eq!(
        shortest_cycle(&loop_back, &find_cycles(&loop_back)[0]),
        vec![id("a"), id("a")]
    );
}

#[test]
fn neighborhood_depth_zero_returns_only_center() {
    let graph = snapshot(&["a", "b"], &[("a", "b")]);

    let result = neighborhood(&graph, &id("a"), 0);

    assert_eq!(result.node_ids, vec![id("a")]);
    assert_eq!(result.edge_count, 0);
}

#[test]
fn neighborhood_traverses_incoming_and_outgoing_edges() {
    let graph = snapshot(&["a", "b", "c", "d"], &[("a", "b"), ("b", "c"), ("d", "b")]);

    let depth_one = neighborhood(&graph, &id("b"), 1);
    let depth_two = neighborhood(&graph, &id("b"), 2);

    assert_eq!(depth_one.node_ids, vec![id("a"), id("b"), id("c"), id("d")]);
    assert_eq!(depth_one.edge_count, 3);
    assert_eq!(depth_two, depth_one);
}

#[test]
fn neighborhood_depth_two_expands_one_more_step_and_sorts_nodes() {
    let graph = snapshot(&["a", "b", "c", "d"], &[("a", "b"), ("b", "c"), ("c", "d")]);

    assert_eq!(
        neighborhood(&graph, &id("a"), 1).node_ids,
        vec![id("a"), id("b")]
    );
    assert_eq!(
        neighborhood(&graph, &id("a"), 2).node_ids,
        vec![id("a"), id("b"), id("c")]
    );
}

#[test]
fn missing_neighborhood_center_returns_empty_result() {
    let result = neighborhood(&snapshot(&["a"], &[]), &id("missing"), 2);

    assert_eq!(result.center, id("missing"));
    assert!(result.node_ids.is_empty());
    assert_eq!(result.edge_count, 0);
}

#[test]
fn dangling_edges_are_ignored_by_algorithms() {
    let graph = snapshot(&["a"], &[("a", "missing")]);

    assert_eq!(
        compute_degrees(&graph).nodes,
        vec![NodeDegree {
            node_id: id("a"),
            fan_in: 0,
            fan_out: 0,
        }]
    );
    assert!(find_cycles(&graph).is_empty());
    assert_eq!(neighborhood(&graph, &id("a"), 1).node_ids, vec![id("a")]);
}

#[test]
fn graph_summary_counts_effective_graph_and_respects_hub_limit() {
    let mut graph = snapshot(
        &["a", "b", "c"],
        &[("a", "b"), ("a", "b"), ("b", "a"), ("c", "a")],
    );
    graph.diagnostics.push(GraphDiagnostic {
        code: "graph.example".to_string(),
        message: "example".to_string(),
        path: None,
    });

    let summary = summarize_graph(&graph, 1);

    assert_eq!(summary.node_count, 3);
    assert_eq!(summary.edge_count, 3);
    assert_eq!(summary.diagnostic_count, 1);
    assert_eq!(summary.cycle_count, 1);
    assert_eq!(summary.top_fan_in.len(), 1);
    assert_eq!(summary.top_fan_out.len(), 1);
}

#[test]
fn graph_summary_is_independent_of_node_and_edge_order() {
    let first = snapshot(&["a", "b", "c"], &[("a", "b"), ("b", "c"), ("c", "a")]);
    let second = snapshot(&["c", "a", "b"], &[("c", "a"), ("a", "b"), ("b", "c")]);

    assert_eq!(summarize_graph(&first, 3), summarize_graph(&second, 3));
}

#[test]
fn blast_radius_collects_transitive_dependents_of_a_seed() {
    // a -> b -> c and d -> b, so changing `c` impacts b (imports c) and the
    // transitive importers a and d.
    let graph = snapshot(&["a", "b", "c", "d"], &[("a", "b"), ("b", "c"), ("d", "b")]);

    let radius = blast_radius(&graph, &[id("c")]);

    assert_eq!(radius.seeds, vec![id("c")]);
    assert_eq!(radius.impacted, vec![id("a"), id("b"), id("d")]);
}

#[test]
fn blast_radius_unions_multiple_seeds_and_excludes_them() {
    let graph = snapshot(&["a", "b", "c", "d"], &[("a", "b"), ("b", "c"), ("d", "b")]);

    let radius = blast_radius(&graph, &[id("c"), id("d")]);

    assert_eq!(radius.seeds, vec![id("c"), id("d")]);
    assert_eq!(radius.impacted, vec![id("a"), id("b")]);
}

#[test]
fn blast_radius_ignores_missing_seeds_and_terminates_on_cycles() {
    let graph = snapshot(&["a", "b"], &[("a", "b"), ("b", "a")]);

    let radius = blast_radius(&graph, &[id("a"), id("missing")]);

    assert_eq!(radius.seeds, vec![id("a")]);
    assert_eq!(radius.impacted, vec![id("b")]);

    let empty = blast_radius(&graph, &[id("missing")]);
    assert!(empty.seeds.is_empty());
    assert!(empty.impacted.is_empty());
}

#[test]
fn directory_dependencies_aggregate_cross_directory_edges() {
    let graph = snapshot(
        &["src/a/x.rs", "src/a/y.rs", "src/b/z.rs"],
        &[
            ("src/a/x.rs", "src/b/z.rs"),
            ("src/a/y.rs", "src/b/z.rs"),
            // Intra-directory edge is cohesion, not a boundary crossing.
            ("src/a/x.rs", "src/a/y.rs"),
        ],
    );

    let deps = directory_dependencies(&graph);

    assert_eq!(
        deps.edges,
        vec![DirectoryDependency {
            from: "src/a".to_string(),
            to: "src/b".to_string(),
            edge_count: 2,
        }]
    );
}

#[test]
fn directory_dependencies_ignore_non_dependency_and_external_edges() {
    let external = GraphNode {
        id: id("external:serde"),
        kind: GraphNodeKind::ExternalDependency,
        label: "serde".to_string(),
        path: None,
    };
    let graph = GraphSnapshot {
        nodes: vec![node("src/a/x.rs"), node("src/b/y.rs"), external],
        edges: vec![
            // A TestOf relationship is not a dependency.
            GraphEdge {
                from: id("src/a/x.rs"),
                to: id("src/b/y.rs"),
                kind: GraphEdgeKind::TestOf,
                provenance: GraphEdgeProvenance::TestHeuristic,
                confidence: GraphEdgeConfidence::High,
            },
            // An external dependency has no directory.
            GraphEdge {
                from: id("src/a/x.rs"),
                to: id("external:serde"),
                kind: GraphEdgeKind::DependsOn,
                provenance: GraphEdgeProvenance::Import,
                confidence: GraphEdgeConfidence::Medium,
            },
        ],
        ..Default::default()
    };

    assert!(directory_dependencies(&graph).edges.is_empty());
}

#[test]
fn directory_dependencies_handle_root_files_and_empty_graph() {
    assert!(
        directory_dependencies(&GraphSnapshot::default())
            .edges
            .is_empty()
    );

    let graph = snapshot(&["main.rs", "src/util.rs"], &[("main.rs", "src/util.rs")]);

    assert_eq!(
        directory_dependencies(&graph).edges,
        vec![DirectoryDependency {
            from: ".".to_string(),
            to: "src".to_string(),
            edge_count: 1,
        }]
    );
}
