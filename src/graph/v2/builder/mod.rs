mod test_edges;

use super::{
    GraphDiagnostic, GraphEdge, GraphEdgeConfidence, GraphEdgeKind, GraphEdgeProvenance, GraphNode,
    GraphNodeId, GraphNodeKind, GraphSnapshot,
};
use crate::graph::resolve_import;
use crate::graph::resolver::normalize_path;
use crate::scan::facts::ScanFacts;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

pub fn graph_snapshot_from_scan(scan: &ScanFacts) -> GraphSnapshot {
    let root = normalize_path(&scan.root_path);
    let mut files = scan
        .files
        .iter()
        .map(|file| {
            let path = normalize_path(&file.path);
            let relative_path = repository_relative_path(&root, &path);
            let label = slash_path(&relative_path);
            let id = GraphNodeId::new(format!("file:{label}"));
            (id, label, path, &file.imports)
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let known_files = files
        .iter()
        .map(|(id, _, path, _)| (path.clone(), id.clone()))
        .collect::<BTreeMap<_, _>>();
    let known_paths = known_files.keys().cloned().collect::<HashSet<_>>();
    let mut nodes = files
        .iter()
        .map(|(id, label, path, _)| {
            (
                id.clone(),
                GraphNode {
                    id: id.clone(),
                    kind: GraphNodeKind::File,
                    label: label.clone(),
                    path: Some(path.clone()),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut edges = Vec::new();
    let mut diagnostics = Vec::new();

    for (source_id, _, source_path, imports) in files {
        for raw_import in imports {
            let raw_import = raw_import.trim();
            if raw_import.is_empty() {
                continue;
            }

            let (target_id, kind, confidence) =
                match resolve_import(raw_import, &source_path, &root, &known_paths) {
                    // A file importing itself carries no dependency information.
                    Some(resolved) if resolved == source_path => continue,
                    Some(resolved) => (
                        known_files
                            .get(&resolved)
                            .cloned()
                            .expect("resolver only returns scanned file paths"),
                        GraphEdgeKind::Imports,
                        GraphEdgeConfidence::High,
                    ),
                    None => {
                        // The shared resolver is authoritative: an unresolved
                        // relative import is a genuine local gap worth surfacing,
                        // while a bare/package import is a real external dependency.
                        let confidence = if is_relative_import(raw_import) {
                            diagnostics.push(GraphDiagnostic {
                            code: "graph-v2.unresolved-import".to_string(),
                            message: format!(
                                "Relative import `{raw_import}` did not resolve to a scanned file"
                            ),
                            path: Some(source_path.clone()),
                        });
                            GraphEdgeConfidence::Low
                        } else {
                            GraphEdgeConfidence::Medium
                        };
                        (
                            external_node_id(&normalize_import(raw_import), &mut nodes),
                            GraphEdgeKind::DependsOn,
                            confidence,
                        )
                    }
                };

            edges.push(GraphEdge {
                from: source_id.clone(),
                to: target_id,
                kind,
                provenance: GraphEdgeProvenance::Import,
                confidence,
            });
        }
    }

    edges.extend(test_edges::test_of_edges(&known_files));

    edges.sort_by(|left, right| {
        left.from
            .cmp(&right.from)
            .then_with(|| left.to.cmp(&right.to))
            .then_with(|| left.kind.cmp(&right.kind))
    });
    // Collapse duplicate relationships by identity, keeping one edge per
    // (from, to, kind) regardless of how many imports produced it.
    edges.dedup_by(|left, right| {
        left.from == right.from && left.to == right.to && left.kind == right.kind
    });

    diagnostics.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.message.cmp(&right.message))
    });
    diagnostics.dedup();

    GraphSnapshot {
        nodes: nodes.into_values().collect(),
        edges,
        diagnostics,
    }
}

fn repository_relative_path(root: &Path, path: &Path) -> PathBuf {
    let relative = path.strip_prefix(root).unwrap_or(path);
    if !relative.as_os_str().is_empty() {
        return relative.to_path_buf();
    }

    path.file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("unknown"))
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn normalize_import(raw_import: &str) -> String {
    raw_import.trim().replace('\\', "/")
}

fn is_relative_import(import: &str) -> bool {
    import.starts_with("./") || import.starts_with("../")
}

fn external_node_id(import: &str, nodes: &mut BTreeMap<GraphNodeId, GraphNode>) -> GraphNodeId {
    let id = GraphNodeId::new(format!("external:{import}"));
    nodes.entry(id.clone()).or_insert_with(|| GraphNode {
        id: id.clone(),
        kind: GraphNodeKind::ExternalDependency,
        label: import.to_string(),
        path: None,
    });
    id
}

#[cfg(test)]
mod tests;
