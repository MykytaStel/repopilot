use super::super::topology;
use crate::graph::v2::{GraphNodeId, GraphSnapshot};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedCyclePaths {
    pub paths: Vec<Vec<GraphNodeId>>,
    pub truncated: bool,
    pub depth_exceeded: bool,
}

const MAX_DEPTH: usize = 512;

pub fn bounded_cycle_paths(
    snapshot: &GraphSnapshot,
    excluded_edges: &BTreeSet<(GraphNodeId, GraphNodeId)>,
    max_cycles: usize,
) -> BoundedCyclePaths {
    if max_cycles == 0 {
        return BoundedCyclePaths {
            paths: Vec::new(),
            truncated: false,
            depth_exceeded: false,
        };
    }
    let topology = topology(snapshot);
    let index_by_id = topology
        .node_ids
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, id)| (id, index))
        .collect::<BTreeMap<_, _>>();
    let adjacency = topology
        .node_ids
        .iter()
        .map(|id| {
            topology.outgoing[id]
                .iter()
                .filter(|target| !excluded_edges.contains(&(id.clone(), (*target).clone())))
                .filter_map(|target| index_by_id.get(target).copied())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut search = Search::new(&topology.node_ids, &adjacency, max_cycles + 1);
    search.run();
    canonicalize(&mut search.paths);
    let truncated = search.paths.len() > max_cycles;
    search.paths.truncate(max_cycles);
    BoundedCyclePaths {
        paths: search.paths,
        truncated,
        depth_exceeded: search.depth_exceeded,
    }
}

fn canonicalize(paths: &mut Vec<Vec<GraphNodeId>>) {
    for path in paths.iter_mut() {
        if let Some(position) = path
            .iter()
            .enumerate()
            .min_by(|left, right| left.1.cmp(right.1))
            .map(|(position, _)| position)
        {
            path.rotate_left(position);
        }
    }
    paths.sort();
    paths.dedup();
}

struct Search<'a> {
    node_ids: &'a [GraphNodeId],
    adjacency: &'a [Vec<usize>],
    state: Vec<u8>,
    stack: Vec<usize>,
    paths: Vec<Vec<GraphNodeId>>,
    limit: usize,
    depth_exceeded: bool,
}

impl<'a> Search<'a> {
    fn new(node_ids: &'a [GraphNodeId], adjacency: &'a [Vec<usize>], limit: usize) -> Self {
        Self {
            node_ids,
            adjacency,
            state: vec![0; node_ids.len()],
            stack: Vec::new(),
            paths: Vec::new(),
            limit,
            depth_exceeded: false,
        }
    }

    fn run(&mut self) {
        for start in 0..self.node_ids.len() {
            if self.paths.len() >= self.limit {
                break;
            }
            if self.state[start] == 0 {
                self.visit(start, 0);
            }
        }
    }

    fn visit(&mut self, node: usize, depth: usize) {
        if depth > MAX_DEPTH {
            self.depth_exceeded = true;
            return;
        }
        self.state[node] = 1;
        self.stack.push(node);
        for &target in &self.adjacency[node] {
            if self.paths.len() >= self.limit {
                break;
            }
            match self.state[target] {
                0 => self.visit(target, depth + 1),
                1 => self.record_cycle(target),
                _ => {}
            }
        }
        self.stack.pop();
        self.state[node] = 2;
    }

    fn record_cycle(&mut self, target: usize) {
        let Some(position) = self.stack.iter().position(|node| *node == target) else {
            return;
        };
        self.paths.push(
            self.stack[position..]
                .iter()
                .map(|node| self.node_ids[*node].clone())
                .collect(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::v2::{
        GraphEdge, GraphEdgeConfidence, GraphEdgeKind, GraphEdgeProvenance, GraphNode,
        GraphNodeKind,
    };
    use std::path::PathBuf;

    #[test]
    fn bounded_paths_are_canonical_and_open() {
        let result = bounded_cycle_paths(
            &snapshot(&[("b", "c"), ("c", "a"), ("a", "b")]),
            &BTreeSet::new(),
            20,
        );
        assert_eq!(result.paths, vec![vec![id("a"), id("b"), id("c")]]);
        assert!(!result.truncated);
        assert!(!result.depth_exceeded);
    }

    #[test]
    fn bounded_paths_report_truncation_without_returning_the_probe() {
        let result = bounded_cycle_paths(
            &snapshot(&[("a", "b"), ("b", "a"), ("c", "d"), ("d", "c")]),
            &BTreeSet::new(),
            1,
        );
        assert_eq!(result.paths, vec![vec![id("a"), id("b")]]);
        assert!(result.truncated);
    }

    fn id(label: &str) -> GraphNodeId {
        GraphNodeId::new(format!("file:{label}"))
    }

    fn snapshot(edges: &[(&str, &str)]) -> GraphSnapshot {
        let labels = edges
            .iter()
            .flat_map(|(from, to)| [*from, *to])
            .collect::<BTreeSet<_>>();
        GraphSnapshot {
            nodes: labels
                .into_iter()
                .map(|label| GraphNode {
                    id: id(label),
                    kind: GraphNodeKind::File,
                    label: label.to_string(),
                    path: Some(PathBuf::from(label)),
                })
                .collect(),
            edges: edges
                .iter()
                .map(|(from, to)| GraphEdge {
                    from: id(from),
                    to: id(to),
                    kind: GraphEdgeKind::Imports,
                    provenance: GraphEdgeProvenance::Import,
                    confidence: GraphEdgeConfidence::High,
                })
                .collect(),
            ..GraphSnapshot::default()
        }
    }
}
