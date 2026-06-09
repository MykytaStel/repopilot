use super::topology;
use crate::graph::v2::{GraphNodeId, GraphSnapshot};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GraphDegreeSummary {
    pub nodes: Vec<NodeDegree>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDegree {
    pub node_id: GraphNodeId,
    pub fan_in: usize,
    pub fan_out: usize,
}

pub fn compute_degrees(snapshot: &GraphSnapshot) -> GraphDegreeSummary {
    let topology = topology(snapshot);
    let mut degrees = topology
        .node_ids
        .into_iter()
        .map(|node_id| {
            (
                node_id.clone(),
                NodeDegree {
                    node_id,
                    fan_in: 0,
                    fan_out: 0,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    for edge in topology.edges {
        degrees
            .get_mut(&edge.from)
            .expect("validated source node")
            .fan_out += 1;
        degrees
            .get_mut(&edge.to)
            .expect("validated target node")
            .fan_in += 1;
    }

    GraphDegreeSummary {
        nodes: degrees.into_values().collect(),
    }
}

pub fn top_fan_in(summary: &GraphDegreeSummary, limit: usize) -> Vec<NodeDegree> {
    if limit == 0 {
        return Vec::new();
    }

    let mut nodes = summary
        .nodes
        .iter()
        .filter(|node| node.fan_in > 0)
        .cloned()
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| {
        right
            .fan_in
            .cmp(&left.fan_in)
            .then_with(|| right.fan_out.cmp(&left.fan_out))
            .then_with(|| left.node_id.cmp(&right.node_id))
    });
    nodes.truncate(limit);
    nodes
}

pub fn top_fan_out(summary: &GraphDegreeSummary, limit: usize) -> Vec<NodeDegree> {
    if limit == 0 {
        return Vec::new();
    }

    let mut nodes = summary
        .nodes
        .iter()
        .filter(|node| node.fan_out > 0)
        .cloned()
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| {
        right
            .fan_out
            .cmp(&left.fan_out)
            .then_with(|| right.fan_in.cmp(&left.fan_in))
            .then_with(|| left.node_id.cmp(&right.node_id))
    });
    nodes.truncate(limit);
    nodes
}
