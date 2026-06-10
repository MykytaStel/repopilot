mod blast_radius;
mod cycles;
mod degree;
mod directory;
mod neighborhood;
mod summary;

pub use blast_radius::{GraphBlastRadius, blast_radius, direct_dependents};
pub use cycles::{GraphCycle, find_cycles, shortest_cycle};
pub use degree::{GraphDegreeSummary, NodeDegree, compute_degrees, top_fan_in, top_fan_out};
pub use directory::{DirectoryDependency, GraphDirectoryDependencies, directory_dependencies};
pub use neighborhood::{GraphNeighborhood, neighborhood};
pub use summary::{GraphV2Summary, summarize_graph};

use super::{GraphEdgeKind, GraphNodeId, GraphSnapshot};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EdgeKey {
    from: GraphNodeId,
    to: GraphNodeId,
    kind: GraphEdgeKind,
}

struct GraphTopology {
    node_ids: Vec<GraphNodeId>,
    node_set: BTreeSet<GraphNodeId>,
    edges: Vec<EdgeKey>,
    outgoing: BTreeMap<GraphNodeId, BTreeSet<GraphNodeId>>,
    incoming: BTreeMap<GraphNodeId, BTreeSet<GraphNodeId>>,
}

fn topology(snapshot: &GraphSnapshot) -> GraphTopology {
    let node_set = snapshot
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    let node_ids = node_set.iter().cloned().collect::<Vec<_>>();
    let edges = snapshot
        .edges
        .iter()
        // Algorithms operate only on nodes present in the snapshot. Dangling
        // endpoints are ignored until graph validation owns that diagnostic.
        .filter(|edge| node_set.contains(&edge.from) && node_set.contains(&edge.to))
        .map(|edge| EdgeKey {
            from: edge.from.clone(),
            to: edge.to.clone(),
            kind: edge.kind,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let mut outgoing = node_ids
        .iter()
        .cloned()
        .map(|id| (id, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    let mut incoming = outgoing.clone();

    for edge in &edges {
        outgoing
            .get_mut(&edge.from)
            .expect("validated source node")
            .insert(edge.to.clone());
        incoming
            .get_mut(&edge.to)
            .expect("validated target node")
            .insert(edge.from.clone());
    }

    GraphTopology {
        node_ids,
        node_set,
        edges,
        outgoing,
        incoming,
    }
}

#[cfg(test)]
mod tests;
