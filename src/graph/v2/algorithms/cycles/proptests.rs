use super::*;
use proptest::prelude::*;
use std::collections::HashMap;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    fn test_tarjan_properties(adjacency in prop::collection::vec(prop::collection::vec(0usize..50, 0..10), 50)) {
        let mut tarjan = Tarjan::new(&adjacency);
        for node in 0..adjacency.len() {
            if tarjan.indices[node].is_none() {
                tarjan.visit(node);
            }
        }

        let mut node_to_scc = HashMap::new();
        for (scc_idx, component) in tarjan.components.iter().enumerate() {
            for &node in component {
                // Property 1: Each node in exactly one SCC
                assert!(node_to_scc.insert(node, scc_idx).is_none(), "Node {} is in multiple SCCs", node);
            }
        }

        // Property 2: All nodes belong to some SCC
        for node in 0..adjacency.len() {
            assert!(node_to_scc.contains_key(&node), "Node {} is not in any SCC", node);
        }
    }
}

#[test]
fn test_deep_recursion_regression() {
    let count = 10000;
    let mut adjacency = vec![vec![]; count];
    // linear chain: 0 -> 1 -> 2 -> ... -> 9999
    for (i, targets) in adjacency.iter_mut().enumerate().take(count - 1) {
        targets.push(i + 1);
    }
    // cycle back: 9999 -> 0
    adjacency[count - 1].push(0);

    let mut tarjan = Tarjan::new(&adjacency);
    tarjan.visit(0);
    assert_eq!(tarjan.components.len(), 1);
    assert_eq!(tarjan.components[0].len(), count);
}
