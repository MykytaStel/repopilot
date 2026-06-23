use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::graph::v2::{build_coupling_graph_snapshot, find_cycles, shortest_cycle};
use crate::graph::{CouplingGraph, without_deferred_edges, without_rust_module_containment_edges};
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn emit_circular_dependency_findings(
    graph: &CouplingGraph,
    prod_files: &HashSet<PathBuf>,
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    let full_cycle_graph = without_rust_module_containment_edges(graph);
    let cycle_graph = without_deferred_edges(&full_cycle_graph);
    let (cycle_snapshot, path_by_id) = build_coupling_graph_snapshot(&cycle_graph);
    let cycles = find_cycles(&cycle_snapshot);

    let mut seen_cycles = BTreeSet::new();
    // Every file that belongs to an eager (non-deferred) cycle. A full-graph SCC
    // that touches any of these is *not* deferred-only — it already contains a
    // real eager cycle that a High finding covers — so it must not also emit a
    // misleading Low "deferred-only" finding for the larger component.
    let mut eager_cycle_files: HashSet<PathBuf> = HashSet::new();
    for cycle in &cycles {
        let members: Vec<PathBuf> = cycle
            .node_ids
            .iter()
            .filter_map(|id| path_by_id.get(id).cloned())
            .collect();

        if !is_prod_cycle(&members, prod_files) {
            continue;
        }

        if seen_cycles.insert(members.clone()) {
            eager_cycle_files.extend(members.iter().cloned());
            let shortest: Vec<PathBuf> = shortest_cycle(&cycle_snapshot, cycle)
                .iter()
                .filter_map(|id| path_by_id.get(id).cloned())
                .collect();
            findings.push(circular_dependency_finding(
                &members,
                &shortest,
                root,
                Severity::High,
                false,
            ));
        }
    }

    if full_cycle_graph.deferred_edges.is_empty() {
        return;
    }

    let (full_snapshot, full_path_by_id) = build_coupling_graph_snapshot(&full_cycle_graph);
    let full_cycles = find_cycles(&full_snapshot);
    for cycle in &full_cycles {
        let members: Vec<PathBuf> = cycle
            .node_ids
            .iter()
            .filter_map(|id| full_path_by_id.get(id).cloned())
            .collect();

        if !is_prod_cycle(&members, prod_files) {
            continue;
        }

        // Mixed component: this SCC only exists in the full graph because a
        // deferred edge widened a component that already has an eager cycle.
        // The eager sub-cycle is reported as High; labelling the whole component
        // "deferred-only" would be wrong (and its shortest cycle could even be
        // the eager one), so skip it.
        if members.iter().any(|path| eager_cycle_files.contains(path)) {
            continue;
        }

        if seen_cycles.insert(members.clone()) {
            let shortest: Vec<PathBuf> = shortest_cycle(&full_snapshot, cycle)
                .iter()
                .filter_map(|id| full_path_by_id.get(id).cloned())
                .collect();
            findings.push(circular_dependency_finding(
                &members,
                &shortest,
                root,
                Severity::Low,
                true,
            ));
        }
    }
}

fn is_prod_cycle(members: &[PathBuf], prod_files: &HashSet<PathBuf>) -> bool {
    !members.is_empty() && members.iter().all(|path| prod_files.contains(path))
}

/// `component` is the full strongly-connected component (all mutually dependent
/// files); `shortest` is the minimal cycle within it, as a closed path
/// (`a -> b -> a`). The finding leads with the actionable minimal cycle and
/// carries the component size as context, instead of repeating the whole
/// component into every evidence snippet.
fn circular_dependency_finding(
    component: &[PathBuf],
    shortest: &[PathBuf],
    root: &Path,
    severity: Severity,
    deferred_only: bool,
) -> Finding {
    let component_size = component.len();

    // The closed path repeats its first node at the end; the distinct files are
    // everything but that trailing repeat.
    let closed: Vec<PathBuf> = if shortest.len() >= 2 {
        shortest
            .iter()
            .map(|path| relative_path(path, root))
            .collect()
    } else {
        component
            .iter()
            .map(|path| relative_path(path, root))
            .collect()
    };
    let cycle_path = closed
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ");
    let distinct: Vec<&PathBuf> = closed.iter().take(closed.len().saturating_sub(1)).collect();
    let distinct = if distinct.is_empty() {
        closed.iter().collect()
    } else {
        distinct
    };

    let context = if component_size > distinct.len() {
        format!(" Part of a strongly-connected component of {component_size} files.")
    } else {
        String::new()
    };
    let deferred_context = if deferred_only {
        " The cycle only exists through deferred imports, so it is informational in strict review."
    } else {
        ""
    };

    let evidence = distinct
        .iter()
        .map(|path| Evidence {
            path: (*path).clone(),
            line_start: 1,
            line_end: None,
            snippet: format!("Cycle: {cycle_path}.{context}{deferred_context}"),
        })
        .collect();
    let title = if deferred_only {
        "Deferred circular dependency detected"
    } else {
        "Circular dependency detected"
    };
    let description_prefix = if deferred_only {
        "A deferred-only circular dependency was detected"
    } else {
        "A circular dependency was detected"
    };

    Finding {
        id: String::new(),
        rule_id: "architecture.circular-dependency".to_string(),
        recommendation: Finding::recommendation_for_rule_id("architecture.circular-dependency"),
        title: title.to_string(),
        description: format!("{description_prefix}: {cycle_path}.{context}{deferred_context}"),
        category: FindingCategory::Architecture,
        severity,
        confidence: Default::default(),
        evidence,
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

fn relative_path(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
