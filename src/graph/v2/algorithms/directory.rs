use crate::graph::v2::{GraphNodeKind, GraphSnapshot};
use std::collections::BTreeMap;

/// A directory-level dependency: the number of file-level dependency edges that
/// cross from one directory into another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryDependency {
    pub from: String,
    pub to: String,
    pub edge_count: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GraphDirectoryDependencies {
    pub edges: Vec<DirectoryDependency>,
}

/// Projects the file-level dependency graph onto directories: each file is
/// mapped to its parent directory and dependency edges are aggregated into
/// directory -> directory edges with counts. Intra-directory edges (cohesion)
/// and edges touching non-file nodes (e.g. external dependencies) are excluded,
/// leaving the cross-directory coupling that boundary and layer analysis builds
/// on. Output is sorted by (from, to) for determinism.
pub fn directory_dependencies(snapshot: &GraphSnapshot) -> GraphDirectoryDependencies {
    let directory_by_file = snapshot
        .nodes
        .iter()
        .filter(|node| node.kind == GraphNodeKind::File)
        .map(|node| (node.id.clone(), directory_of(&node.label)))
        .collect::<BTreeMap<_, _>>();

    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for edge in &snapshot.edges {
        if !edge.kind.is_dependency() {
            continue;
        }
        let (Some(from_dir), Some(to_dir)) = (
            directory_by_file.get(&edge.from),
            directory_by_file.get(&edge.to),
        ) else {
            continue;
        };
        if from_dir != to_dir {
            *counts
                .entry((from_dir.clone(), to_dir.clone()))
                .or_insert(0) += 1;
        }
    }

    let edges = counts
        .into_iter()
        .map(|((from, to), edge_count)| DirectoryDependency {
            from,
            to,
            edge_count,
        })
        .collect();

    GraphDirectoryDependencies { edges }
}

/// The parent directory of a repository-relative slash path, or `"."` for files
/// at the repository root.
fn directory_of(label: &str) -> String {
    match label.rfind('/') {
        Some(index) => label[..index].to_string(),
        None => ".".to_string(),
    }
}
