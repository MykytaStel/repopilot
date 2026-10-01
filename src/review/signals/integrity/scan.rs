//! One preorder pass over one side of a changed file. It collects markers,
//! suppressions, test cases, and same-file helpers. Each assertion and helper
//! call is credited to the test and helper bodies that contain it, from the
//! innermost outward, stopping at the first test: a nested test's checks
//! belong to it, not to the group around it.

use super::accounting::TestFacts;
use super::helpers::{self, Body};
use super::syntax::for_each_node;
use crate::review::signals::content::ReviewSource;
use crate::review::signals::tables::{IntegrityTables, TestMarker};
use std::collections::{BTreeMap, BTreeSet};

/// What one side of a changed file contains. Markers and test cases are
/// collected only in test scope; suppressions everywhere.
#[derive(Debug, Default)]
pub(super) struct Scan {
    pub(super) markers: Vec<TestMarker>,
    pub(super) tests: Vec<TestFacts>,
    pub(super) suppressions: Vec<(String, usize)>,
}

#[derive(Clone, Copy)]
enum Owner {
    Test(usize),
    Helper(usize),
}

/// A test or helper whose subtree the pass is inside.
struct Open {
    depth: usize,
    owner: Owner,
}

pub(super) fn scan(
    source: &ReviewSource,
    tables: &IntegrityTables,
    test_scope: bool,
) -> Option<Scan> {
    let tree = source.tree()?;
    let content = source.content();
    let root = tree.root_node();
    let known = if test_scope {
        helpers::definition_names(root, content)
    } else {
        BTreeSet::new()
    };
    let mut found = Scan::default();
    let mut helper_bodies: Vec<(String, Body)> = Vec::new();
    let mut open: Vec<Open> = Vec::new();
    // Depths of the loops enclosing the current node.
    let mut loops: Vec<usize> = Vec::new();
    for_each_node(root, |node, depth| {
        while open.last().is_some_and(|outer| outer.depth >= depth) {
            open.pop();
        }
        while loops.last().is_some_and(|&start| start >= depth) {
            loops.pop();
        }
        if let Some(label) = (tables.suppression)(node, content) {
            found
                .suppressions
                .push((label, node.start_position().row + 1));
        }
        if !test_scope {
            return true;
        }
        if let Some(marker) = (tables.test_marker)(node, content) {
            found.markers.push(marker);
        }
        if let Some(name) = (tables.test_case)(node, content) {
            found.tests.push(TestFacts::of(node, content, name));
            let owner = Owner::Test(found.tests.len() - 1);
            open.push(Open { depth, owner });
            return true;
        }
        if let Some(name) = helpers::definition_name(node, content) {
            helper_bodies.push((name, Body::default()));
            let owner = Owner::Helper(helper_bodies.len() - 1);
            open.push(Open { depth, owner });
            return true;
        }
        if open.is_empty() {
            return true;
        }
        let callee = helpers::callee_name(node, content);
        let called = callee.filter(|name| known.contains(*name));
        let asserts = (tables.is_assertion)(node, content);
        if asserts || called.is_some() {
            for body in open.iter().rev() {
                let in_loop = loops.last().is_some_and(|&start| start > body.depth);
                let target = match body.owner {
                    Owner::Test(index) => found.tests[index].body_mut(),
                    Owner::Helper(index) => &mut helper_bodies[index].1,
                };
                if asserts {
                    target.add_assertion(in_loop);
                }
                if let Some(name) = called {
                    target.add_call(name, in_loop);
                }
                if matches!(body.owner, Owner::Test(_)) {
                    break;
                }
            }
        }
        if helpers::iterates(node, callee) {
            loops.push(depth);
        }
        true
    });
    let mut helper_map: BTreeMap<String, Body> = BTreeMap::new();
    for (name, body) in helper_bodies {
        helpers::remember(&mut helper_map, name, body);
    }
    for test in &mut found.tests {
        test.include_helpers(&helper_map);
    }
    Some(found)
}
