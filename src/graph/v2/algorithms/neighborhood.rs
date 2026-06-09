use super::topology;
use crate::graph::v2::{GraphNodeId, GraphSnapshot};
use std::collections::{BTreeSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNeighborhood {
    pub center: GraphNodeId,
    pub node_ids: Vec<GraphNodeId>,
    pub edge_count: usize,
}

pub fn neighborhood(
    snapshot: &GraphSnapshot,
    center: &GraphNodeId,
    depth: usize,
) -> GraphNeighborhood {
    let topology = topology(snapshot);
    if !topology.node_set.contains(center) {
        return GraphNeighborhood {
            center: center.clone(),
            node_ids: Vec::new(),
            edge_count: 0,
        };
    }

    let mut visited = BTreeSet::from([center.clone()]);
    let mut queue = VecDeque::from([(center.clone(), 0usize)]);

    while let Some((node, distance)) = queue.pop_front() {
        if distance >= depth {
            continue;
        }

        let neighbors = topology.outgoing[&node]
            .iter()
            .chain(topology.incoming[&node].iter())
            .cloned()
            .collect::<BTreeSet<_>>();
        for neighbor in neighbors {
            if visited.insert(neighbor.clone()) {
                queue.push_back((neighbor, distance + 1));
            }
        }
    }

    let edge_count = topology
        .edges
        .iter()
        .filter(|edge| visited.contains(&edge.from) && visited.contains(&edge.to))
        .count();

    GraphNeighborhood {
        center: center.clone(),
        node_ids: visited.into_iter().collect(),
        edge_count,
    }
}
