use super::{compute_degrees, find_cycles, top_fan_in, top_fan_out, topology};
use crate::graph::v2::{GraphSnapshot, NodeDegree};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GraphV2Summary {
    pub node_count: usize,
    pub edge_count: usize,
    pub diagnostic_count: usize,
    pub cycle_count: usize,
    pub top_fan_in: Vec<NodeDegree>,
    pub top_fan_out: Vec<NodeDegree>,
}

pub fn summarize_graph(snapshot: &GraphSnapshot, hub_limit: usize) -> GraphV2Summary {
    let topology = topology(snapshot);
    let degrees = compute_degrees(snapshot);
    let cycles = find_cycles(snapshot);

    GraphV2Summary {
        node_count: topology.node_ids.len(),
        edge_count: topology.edges.len(),
        diagnostic_count: snapshot.diagnostics.len(),
        cycle_count: cycles.len(),
        top_fan_in: top_fan_in(&degrees, hub_limit),
        top_fan_out: top_fan_out(&degrees, hub_limit),
    }
}
