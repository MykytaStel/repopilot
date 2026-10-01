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
    let mut found = Scan::default();
    let mut helper_bodies: Vec<(String, Body)> = Vec::new();
    let mut open: Vec<Open> = Vec::new();
    // Depths of the loops enclosing the current node.
    let mut loops: Vec<usize> = Vec::new();
    let relevant = relevant_kinds(tree);
    for_each_node(root, |node, depth| {
        while open.last().is_some_and(|outer| outer.depth >= depth) {
            open.pop();
        }
        while loops.last().is_some_and(|&start| start >= depth) {
            loops.pop();
        }
        if !relevant
            .get(usize::from(node.kind_id()))
            .copied()
            .unwrap_or(false)
        {
            return true;
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
        // Callee names are resolved against the file's helpers after the walk.
        let called = callee.map(|name| {
            (
                name.as_ptr() as usize - content.as_ptr() as usize,
                name.len(),
            )
        });
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
                if let Some((start, len)) = called {
                    target.add_pending_call(start, len, in_loop);
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
    let names: BTreeSet<String> = helper_bodies.iter().map(|(name, _)| name.clone()).collect();
    for (_, body) in &mut helper_bodies {
        body.settle(content, &names);
    }
    for test in &mut found.tests {
        test.body_mut().settle(content, &names);
    }
    let mut helper_map: BTreeMap<String, Body> = BTreeMap::new();
    for (name, body) in helper_bodies {
        helpers::remember(&mut helper_map, name, body);
    }
    for test in &mut found.tests {
        test.include_helpers(&helper_map);
    }
    Some(found)
}

/// Every node kind a recognizer, helper, or loop check can match on entry, in
/// any supported grammar. Other nodes only move the walk along.
const RELEVANT_KINDS: &[&str] = &[
    "call_expression",
    "call",
    "comment",
    "line_comment",
    "block_comment",
    "decorator",
    "assignment",
    "assert_statement",
    "function_definition",
    "function_declaration",
    "generator_function_declaration",
    "method_definition",
    "method_declaration",
    "variable_declarator",
    "function_item",
    "attribute_item",
    "inner_attribute_item",
    "macro_invocation",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "do_statement",
    "for_expression",
    "while_expression",
    "loop_expression",
];

/// A lookup by `kind_id`, so the walk compares a number per node instead of
/// reading every node's kind name.
fn relevant_kinds(tree: &tree_sitter::Tree) -> Vec<bool> {
    let language = tree.language();
    (0..language.node_kind_count())
        .map(|id| {
            u16::try_from(id)
                .ok()
                .and_then(|id| language.node_kind_for_id(id))
                .is_some_and(|name| RELEVANT_KINDS.contains(&name))
        })
        .collect()
}
