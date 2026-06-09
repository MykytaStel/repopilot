use super::topology;
use crate::graph::v2::{GraphNodeId, GraphSnapshot};
use std::collections::{BTreeSet, VecDeque};

/// The transitive dependents of a set of changed nodes: every node that can
/// reach a seed by following dependency edges (i.e. the importers, directly or
/// transitively). This is the blast radius of a change to the seeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphBlastRadius {
    /// Provided seeds that exist in the snapshot, sorted and deduplicated.
    pub seeds: Vec<GraphNodeId>,
    /// Transitive dependents of the seeds, sorted, excluding the seeds.
    pub impacted: Vec<GraphNodeId>,
}

/// Walks dependency edges in reverse (`incoming`) from every seed, so the result
/// is "what is affected if these nodes change". Seeds missing from the snapshot
/// are ignored; cycles terminate because nodes are visited at most once.
pub fn blast_radius(snapshot: &GraphSnapshot, seeds: &[GraphNodeId]) -> GraphBlastRadius {
    let topology = topology(snapshot);
    let present_seeds = seeds
        .iter()
        .filter(|seed| topology.node_set.contains(seed))
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut visited = present_seeds.clone();
    let mut queue = present_seeds.iter().cloned().collect::<VecDeque<_>>();

    while let Some(node) = queue.pop_front() {
        for dependent in &topology.incoming[&node] {
            if visited.insert(dependent.clone()) {
                queue.push_back(dependent.clone());
            }
        }
    }

    let impacted = visited
        .into_iter()
        .filter(|node| !present_seeds.contains(node))
        .collect::<Vec<_>>();

    GraphBlastRadius {
        seeds: present_seeds.into_iter().collect(),
        impacted,
    }
}
