use crate::scan::types::CouplingGraph;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

thread_local! {
    static CYCLE_DETECTION_DEPTH_EXCEEDED: Cell<bool> = const { Cell::new(false) };
}

pub fn was_cycle_detection_depth_exceeded() -> bool {
    CYCLE_DETECTION_DEPTH_EXCEEDED.with(|c| c.get())
}

pub fn clear_cycle_detection_depth_exceeded() {
    CYCLE_DETECTION_DEPTH_EXCEEDED.with(|c| c.set(false));
}

pub(crate) fn mark_cycle_detection_depth_exceeded() {
    CYCLE_DETECTION_DEPTH_EXCEEDED.with(|c| c.set(true));
}

const MAX_DFS_DEPTH: usize = 512;

pub fn detect_cycles(graph: &CouplingGraph) -> Vec<Vec<PathBuf>> {
    detect_cycles_bounded(graph, usize::MAX)
}

pub fn detect_cycles_bounded(graph: &CouplingGraph, max_cycles: usize) -> Vec<Vec<PathBuf>> {
    clear_cycle_detection_depth_exceeded();
    if max_cycles == 0 {
        return Vec::new();
    }

    let nodes: Vec<&PathBuf> = graph.nodes.iter().collect();
    let n = nodes.len();

    let index: HashMap<&PathBuf, usize> = nodes.iter().enumerate().map(|(i, p)| (*p, i)).collect();

    // Deferred/non-runtime edges are excluded: Python function-body imports run
    // only when called, and TS/JS type-only imports are erased.
    let adj: Vec<Vec<usize>> = nodes
        .iter()
        .map(|node| {
            let deferred = graph.deferred_edges.get(*node);
            graph
                .edges
                .get(*node)
                .map(|targets| {
                    targets
                        .iter()
                        .filter(|t| !deferred.is_some_and(|d| d.contains(*t)))
                        .filter_map(|t| index.get(t).copied())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    let mut state = vec![0u8; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut cycles: Vec<Vec<PathBuf>> = Vec::new();
    {
        let mut dfs_state = CycleDfs {
            adj: &adj,
            state: &mut state,
            stack: &mut stack,
            cycles: &mut cycles,
            nodes: &nodes,
            max_cycles,
        };

        for start in 0..n {
            if dfs_state.cycles.len() >= max_cycles {
                break;
            }
            if dfs_state.state[start] == 0 {
                dfs_state.visit(start, 0);
            }
        }
    }

    for cycle in &mut cycles {
        if let Some(min_pos) = cycle
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.cmp(b.1))
            .map(|(i, _)| i)
        {
            cycle.rotate_left(min_pos);
        }
    }

    cycles.sort();
    cycles.dedup();
    cycles.truncate(max_cycles);
    cycles
}

pub fn without_rust_module_containment_edges(graph: &CouplingGraph) -> CouplingGraph {
    let edges = graph
        .edges
        .iter()
        .map(|(source, targets)| {
            (
                source.clone(),
                targets
                    .iter()
                    .filter(|target| !is_rust_module_containment_edge(source, target))
                    .cloned()
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect();

    CouplingGraph {
        edges,
        deferred_edges: graph.deferred_edges.clone(),
        nodes: graph.nodes.clone(),
    }
}

/// Returns a copy of `graph` with every deferred / non-runtime edge removed,
/// for cycle detection only.
pub fn without_deferred_edges(graph: &CouplingGraph) -> CouplingGraph {
    if graph.deferred_edges.is_empty() {
        return graph.clone();
    }
    let edges = graph
        .edges
        .iter()
        .map(|(source, targets)| {
            let deferred = graph.deferred_edges.get(source);
            (
                source.clone(),
                targets
                    .iter()
                    .filter(|target| !deferred.is_some_and(|d| d.contains(*target)))
                    .cloned()
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect();

    CouplingGraph {
        edges,
        deferred_edges: BTreeMap::new(),
        nodes: graph.nodes.clone(),
    }
}

fn is_rust_module_containment_edge(source: &Path, target: &Path) -> bool {
    if source.extension().and_then(|ext| ext.to_str()) != Some("rs")
        || target.extension().and_then(|ext| ext.to_str()) != Some("rs")
    {
        return false;
    }

    let Some(module_dir) = rust_declared_module_dir(source) else {
        return false;
    };

    if target.parent() == Some(module_dir.as_path()) && target.file_name() != source.file_name() {
        return true;
    }

    target.file_name().and_then(|name| name.to_str()) == Some("mod.rs")
        && target.parent().and_then(Path::parent) == Some(module_dir.as_path())
}

fn rust_declared_module_dir(source: &Path) -> Option<PathBuf> {
    match source.file_name().and_then(|name| name.to_str()) {
        Some("lib.rs" | "main.rs" | "mod.rs") => source.parent().map(Path::to_path_buf),
        Some(_) => Some(source.with_extension("")),
        None => None,
    }
}

struct CycleDfs<'a, 'b> {
    adj: &'a [Vec<usize>],
    state: &'a mut Vec<u8>,
    stack: &'a mut Vec<usize>,
    cycles: &'a mut Vec<Vec<PathBuf>>,
    nodes: &'a [&'b PathBuf],
    max_cycles: usize,
}

impl CycleDfs<'_, '_> {
    fn visit(&mut self, node: usize, depth: usize) {
        if self.cycles.len() >= self.max_cycles {
            return;
        }
        if depth > MAX_DFS_DEPTH {
            CYCLE_DETECTION_DEPTH_EXCEEDED.with(|c| c.set(true));
            self.state[node] = 2;
            return;
        }

        self.state[node] = 1;
        self.stack.push(node);

        for &neighbor in &self.adj[node] {
            if self.cycles.len() >= self.max_cycles {
                break;
            }
            match self.state[neighbor] {
                1 => {
                    if let Some(pos) = self.stack.iter().position(|&n| n == neighbor) {
                        let cycle = self.stack[pos..]
                            .iter()
                            .map(|&i| self.nodes[i].clone())
                            .collect();
                        self.cycles.push(cycle);
                    }
                }
                0 => self.visit(neighbor, depth + 1),
                _ => {}
            }
        }

        self.stack.pop();
        self.state[node] = 2;
    }
}
