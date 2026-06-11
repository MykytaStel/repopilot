pub mod context;
pub mod imports;
pub mod resolver;
pub mod v2;

mod coupling_metrics;
mod resolution_stats;

pub use coupling_metrics::coupling_file_metrics;
pub use imports::extract_imports;
pub use resolution_stats::ImportResolutionStats;
pub use resolver::resolve_import;

use crate::scan::facts::ScanFacts;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

pub use crate::scan::types::CouplingGraph;

// ── Data structures ───────────────────────────────────────────────────────────

pub struct FileMetrics {
    pub path: PathBuf,
    pub fan_in: usize,
    pub fan_out: usize,
    /// instability = fan_out / (fan_in + fan_out); 0.0 when both are zero.
    pub instability: f32,
}

// ── Graph construction ────────────────────────────────────────────────────────

pub fn build_coupling_graph(facts: &ScanFacts, root: &Path) -> CouplingGraph {
    build_coupling_graph_with_resolution(facts, root).0
}

/// Builds the coupling graph and, alongside it, the imports the resolver could
/// not map to scanned files. The graph carries only proven edges; the stats
/// tell absence-based consumers (dead modules, fan-in metrics) where the graph
/// is provably incomplete.
pub fn build_coupling_graph_with_resolution(
    facts: &ScanFacts,
    root: &Path,
) -> (CouplingGraph, ImportResolutionStats) {
    let known_file_by_normalized: HashMap<PathBuf, PathBuf> = facts
        .files
        .iter()
        .map(|file| (resolver::normalize_path(&file.path), file.path.clone()))
        .collect();
    let known_files: HashSet<PathBuf> = known_file_by_normalized.keys().cloned().collect();

    let mut edges: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut resolution = ImportResolutionStats::default();

    for file in &facts.files {
        let source = file.path.clone();
        let normalized_source = resolver::normalize_path(&source);
        // Insert into edges (one clone); nodes is derived from edges keys afterwards.
        let outgoing = edges.entry(source.clone()).or_default();

        for raw in &file.imports {
            match resolve_import(raw, &normalized_source, root, &known_files) {
                Some(target) if target != normalized_source => {
                    outgoing.insert(
                        known_file_by_normalized
                            .get(&target)
                            .cloned()
                            .unwrap_or(target),
                    );
                }
                // A file importing itself carries no dependency information.
                Some(_) => {}
                None => {
                    if resolution_stats::is_relative_import(raw.trim()) {
                        resolution.record(&source, raw.trim());
                    }
                }
            }
        }
    }

    // Build nodes from all sources (edge origins + edge targets).
    let mut nodes: BTreeSet<PathBuf> = edges.keys().cloned().collect();
    for targets in edges.values() {
        nodes.extend(targets.iter().cloned());
    }

    (CouplingGraph { edges, nodes }, resolution)
}

// ── Metrics ───────────────────────────────────────────────────────────────────

pub fn compute_metrics(graph: &CouplingGraph) -> Vec<FileMetrics> {
    // Single pass: accumulate fan_out and fan_in from edges without pre-initialising maps.
    let mut fan_out: HashMap<&PathBuf, usize> = HashMap::new();
    let mut fan_in: HashMap<&PathBuf, usize> = HashMap::new();

    for (from, targets) in &graph.edges {
        fan_out.insert(from, targets.len());
        for target in targets {
            *fan_in.entry(target).or_insert(0) += 1;
        }
    }

    graph
        .nodes
        .iter()
        .map(|path| {
            let fo = fan_out.get(path).copied().unwrap_or(0);
            let fi = fan_in.get(path).copied().unwrap_or(0);
            let instability = if fo + fi == 0 {
                0.0_f32
            } else {
                fo as f32 / (fi + fo) as f32
            };
            FileMetrics {
                path: path.clone(),
                fan_in: fi,
                fan_out: fo,
                instability,
            }
        })
        .collect()
}

use std::cell::Cell;

thread_local! {
    static CYCLE_DETECTION_DEPTH_EXCEEDED: Cell<bool> = const { Cell::new(false) };
}

pub fn was_cycle_detection_depth_exceeded() -> bool {
    CYCLE_DETECTION_DEPTH_EXCEEDED.with(|c| c.get())
}

pub fn clear_cycle_detection_depth_exceeded() {
    CYCLE_DETECTION_DEPTH_EXCEEDED.with(|c| c.set(false));
}

// ── Cycle detection ───────────────────────────────────────────────────────────

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

    // Map each node to a stable integer index
    let index: HashMap<&PathBuf, usize> = nodes.iter().enumerate().map(|(i, p)| (*p, i)).collect();

    // Adjacency list using indices
    let adj: Vec<Vec<usize>> = nodes
        .iter()
        .map(|node| {
            graph
                .edges
                .get(*node)
                .map(|targets| {
                    targets
                        .iter()
                        .filter_map(|t| index.get(t).copied())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    // 0 = unvisited, 1 = in-progress, 2 = done
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

    // Canonicalize each cycle: rotate so the smallest path is first
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
                    // Back edge -> cycle; extract the loop from the current stack.
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn no_cycle_in_dag() {
        let graph = graph_from_edges(&[("a.rs", "b.rs"), ("b.rs", "c.rs"), ("a.rs", "c.rs")]);

        let cycles = detect_cycles(&graph);

        assert!(cycles.is_empty());
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
    fn disconnected_graph_no_cycle() {
        let graph = graph_from_edges(&[("a.rs", "b.rs"), ("c.rs", "d.rs")]);

        let cycles = detect_cycles(&graph);

        assert!(cycles.is_empty());
    }

    #[test]
    fn compute_metrics_fan_in_and_fan_out() {
        // a → b → c; a → c
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
}
