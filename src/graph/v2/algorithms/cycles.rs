use super::topology;
use crate::graph::v2::{GraphNodeId, GraphSnapshot};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphCycle {
    pub node_ids: Vec<GraphNodeId>,
}

pub fn find_cycles(snapshot: &GraphSnapshot) -> Vec<GraphCycle> {
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
                .filter_map(|target| index_by_id.get(target).copied())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut tarjan = Tarjan::new(&adjacency);

    for node in 0..topology.node_ids.len() {
        if tarjan.indices[node].is_none() {
            tarjan.visit(node);
        }
    }

    let mut cycles = tarjan
        .components
        .into_iter()
        .filter(|component| {
            component.len() > 1
                || component
                    .first()
                    .is_some_and(|node| adjacency[*node].contains(node))
        })
        .map(|component| {
            let mut node_ids = component
                .into_iter()
                .map(|node| topology.node_ids[node].clone())
                .collect::<Vec<_>>();
            node_ids.sort();
            GraphCycle { node_ids }
        })
        .collect::<Vec<_>>();

    cycles.sort_by(|left, right| {
        right
            .node_ids
            .len()
            .cmp(&left.node_ids.len())
            .then_with(|| left.node_ids.first().cmp(&right.node_ids.first()))
            .then_with(|| left.node_ids.cmp(&right.node_ids))
    });
    cycles
}

struct Tarjan<'a> {
    adjacency: &'a [Vec<usize>],
    next_index: usize,
    indices: Vec<Option<usize>>,
    lowlink: Vec<usize>,
    stack: Vec<usize>,
    on_stack: Vec<bool>,
    components: Vec<Vec<usize>>,
}

impl<'a> Tarjan<'a> {
    fn new(adjacency: &'a [Vec<usize>]) -> Self {
        let node_count = adjacency.len();
        Self {
            adjacency,
            next_index: 0,
            indices: vec![None; node_count],
            lowlink: vec![0; node_count],
            stack: Vec::new(),
            on_stack: vec![false; node_count],
            components: Vec::new(),
        }
    }

    fn visit(&mut self, node: usize) {
        let index = self.next_index;
        self.next_index += 1;
        self.indices[node] = Some(index);
        self.lowlink[node] = index;
        self.stack.push(node);
        self.on_stack[node] = true;

        for &neighbor in &self.adjacency[node] {
            if self.indices[neighbor].is_none() {
                self.visit(neighbor);
                self.lowlink[node] = self.lowlink[node].min(self.lowlink[neighbor]);
            } else if self.on_stack[neighbor] {
                self.lowlink[node] = self.lowlink[node]
                    .min(self.indices[neighbor].expect("visited neighbor has an index"));
            }
        }

        if self.lowlink[node] != index {
            return;
        }

        let mut component = Vec::new();
        loop {
            let member = self.stack.pop().expect("current SCC contains its root");
            self.on_stack[member] = false;
            component.push(member);
            if member == node {
                break;
            }
        }
        self.components.push(component);
    }
}
