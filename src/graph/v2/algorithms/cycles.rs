use super::topology;
use crate::graph::v2::{GraphNodeId, GraphSnapshot};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphCycle {
    pub node_ids: Vec<GraphNodeId>,
}

/// Cap on how many start nodes the shortest-cycle search tries, bounding cost on
/// pathologically large strongly-connected components. Members are sorted, so
/// the search stays deterministic.
const MAX_SHORTEST_CYCLE_STARTS: usize = 64;

/// The shortest directed cycle within a strongly-connected component, returned
/// as a closed path (e.g. `a -> b -> a` is `[a, b, a]`). Useful as the
/// actionable headline for an `architecture.circular-dependency` finding when
/// the full component is too large to read. Falls back to the (sorted) member
/// list if no cycle can be reconstructed.
pub fn shortest_cycle(snapshot: &GraphSnapshot, cycle: &GraphCycle) -> Vec<GraphNodeId> {
    let members = &cycle.node_ids;
    let index_by_id: BTreeMap<&GraphNodeId, usize> = members
        .iter()
        .enumerate()
        .map(|(index, id)| (id, index))
        .collect();

    let mut adjacency = vec![Vec::new(); members.len()];
    for edge in &snapshot.edges {
        if let (Some(&from), Some(&to)) = (index_by_id.get(&edge.from), index_by_id.get(&edge.to)) {
            adjacency[from].push(to);
        }
    }
    for targets in &mut adjacency {
        targets.sort_unstable();
        targets.dedup();
    }

    let mut best: Option<Vec<usize>> = None;
    for start in 0..members.len().min(MAX_SHORTEST_CYCLE_STARTS) {
        if let Some(path) = shortest_cycle_from(start, &adjacency)
            && best
                .as_ref()
                .is_none_or(|current| path.len() < current.len())
        {
            best = Some(path);
        }
    }

    match best {
        Some(path) => path
            .into_iter()
            .map(|index| members[index].clone())
            .collect(),
        None => members.clone(),
    }
}

/// Breadth-first search for the shortest cycle returning to `start`, as a closed
/// path of node indices `[start, ..., start]`.
fn shortest_cycle_from(start: usize, adjacency: &[Vec<usize>]) -> Option<Vec<usize>> {
    let mut parent = vec![usize::MAX; adjacency.len()];
    let mut visited = vec![false; adjacency.len()];
    visited[start] = true;
    let mut queue = VecDeque::from([start]);

    while let Some(node) = queue.pop_front() {
        for &next in &adjacency[node] {
            if next == start {
                let mut chain = Vec::new();
                let mut current = node;
                while current != start {
                    chain.push(current);
                    current = parent[current];
                }
                chain.reverse();
                let mut path = Vec::with_capacity(chain.len() + 2);
                path.push(start);
                path.extend(chain);
                path.push(start);
                return Some(path);
            }
            if !visited[next] {
                visited[next] = true;
                parent[next] = node;
                queue.push_back(next);
            }
        }
    }

    None
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
