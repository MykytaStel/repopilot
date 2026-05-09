pub mod imports;
pub mod resolver;

pub use imports::extract_imports;
pub use resolver::resolve_import;

use crate::scan::facts::ScanFacts;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ── Data structures ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CouplingGraph {
    /// Outgoing edges: source file → set of files it imports.
    pub edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    /// Every scanned file, including those with no edges.
    pub nodes: BTreeSet<PathBuf>,
}

pub struct FileMetrics {
    pub path: PathBuf,
    pub fan_in: usize,
    pub fan_out: usize,
    /// instability = fan_out / (fan_in + fan_out); 0.0 when both are zero.
    pub instability: f32,
}

// ── Graph construction ────────────────────────────────────────────────────────

pub fn build_coupling_graph(facts: &ScanFacts, root: &Path) -> CouplingGraph {
    let known_files: BTreeSet<PathBuf> = facts.files.iter().map(|f| f.path.clone()).collect();

    let mut edges: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();

    for file in &facts.files {
        // Insert into edges (one clone); nodes is derived from edges keys afterwards.
        let outgoing = edges.entry(file.path.clone()).or_default();

        for raw in &file.imports {
            if let Some(target) = resolve_import(raw, &file.path, root, &known_files) {
                if target != file.path {
                    outgoing.insert(target);
                }
            }
        }
    }

    // Build nodes from all sources (edge origins + edge targets).
    let mut nodes: BTreeSet<PathBuf> = edges.keys().cloned().collect();
    for targets in edges.values() {
        nodes.extend(targets.iter().cloned());
    }

    CouplingGraph { edges, nodes }
}

// ── Metrics ───────────────────────────────────────────────────────────────────

pub fn compute_metrics(graph: &CouplingGraph) -> Vec<FileMetrics> {
    // Single pass: accumulate fan_out and fan_in from edges without pre-initialising maps.
    let mut fan_out: BTreeMap<&PathBuf, usize> = BTreeMap::new();
    let mut fan_in: BTreeMap<&PathBuf, usize> = BTreeMap::new();

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

// ── Cycle detection ───────────────────────────────────────────────────────────

const MAX_DFS_DEPTH: usize = 512;

pub fn detect_cycles(graph: &CouplingGraph) -> Vec<Vec<PathBuf>> {
    let nodes: Vec<&PathBuf> = graph.nodes.iter().collect();
    let n = nodes.len();

    // Map each node to a stable integer index
    let index: BTreeMap<&PathBuf, usize> = nodes.iter().enumerate().map(|(i, p)| (*p, i)).collect();

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

    for start in 0..n {
        if state[start] == 0 {
            dfs(start, &adj, &mut state, &mut stack, &mut cycles, &nodes, 0);
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
    cycles
}

fn dfs(
    node: usize,
    adj: &[Vec<usize>],
    state: &mut Vec<u8>,
    stack: &mut Vec<usize>,
    cycles: &mut Vec<Vec<PathBuf>>,
    nodes: &[&PathBuf],
    depth: usize,
) {
    if depth > MAX_DFS_DEPTH {
        state[node] = 2;
        return;
    }

    state[node] = 1;
    stack.push(node);

    for &neighbor in &adj[node] {
        match state[neighbor] {
            1 => {
                // Back edge → cycle; extract the loop from the current stack
                if let Some(pos) = stack.iter().position(|&n| n == neighbor) {
                    let cycle = stack[pos..].iter().map(|&i| nodes[i].clone()).collect();
                    cycles.push(cycle);
                }
            }
            0 => dfs(neighbor, adj, state, stack, cycles, nodes, depth + 1),
            _ => {}
        }
    }

    stack.pop();
    state[node] = 2;
}
