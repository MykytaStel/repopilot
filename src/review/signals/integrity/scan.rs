//! One preorder pass over one side of a changed file. It collects markers,
//! suppressions, test cases, and same-file helpers. Each assertion and helper
//! call is credited to the test and helper bodies that contain it, from the
//! innermost outward, stopping at the first test: a nested test's checks
//! belong to it, not to the group around it.

use super::accounting::TestFacts;
use super::helpers::{self, Body};
use super::syntax::for_each_node;
use crate::review::signals::content::ReviewSource;
use crate::review::signals::tables::{AssertionMatcher, IntegrityTables, TestMarker};
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
    let mut walker = Walker::new(tree, source.content(), tables, test_scope);
    for_each_node(tree.root_node(), |node, depth| walker.visit(node, depth));
    Some(walker.finish())
}

struct Walker<'a> {
    content: &'a str,
    tables: &'a IntegrityTables,
    test_scope: bool,
    relevant: Vec<bool>,
    found: Scan,
    helper_bodies: Vec<(String, Body)>,
    open: Vec<Open>,
    loops: Vec<usize>,
}

impl<'a> Walker<'a> {
    fn new(
        tree: &tree_sitter::Tree,
        content: &'a str,
        tables: &'a IntegrityTables,
        test_scope: bool,
    ) -> Self {
        Self {
            content,
            tables,
            test_scope,
            relevant: relevant_kinds(tree),
            found: Scan::default(),
            helper_bodies: Vec::new(),
            open: Vec::new(),
            loops: Vec::new(),
        }
    }

    fn visit(&mut self, node: tree_sitter::Node<'_>, depth: usize) -> bool {
        while self.open.last().is_some_and(|outer| outer.depth >= depth) {
            self.open.pop();
        }
        while self.loops.last().is_some_and(|&start| start >= depth) {
            self.loops.pop();
        }
        if !self
            .relevant
            .get(usize::from(node.kind_id()))
            .copied()
            .unwrap_or(false)
        {
            return true;
        }
        if let Some(label) = (self.tables.suppression)(node, self.content) {
            self.found
                .suppressions
                .push((label, node.start_position().row + 1));
        }
        if !self.test_scope {
            return true;
        }
        if let Some(marker) = (self.tables.test_marker)(node, self.content) {
            self.found.markers.push(marker);
        }
        if let Some(name) = (self.tables.test_case)(node, self.content) {
            self.found
                .tests
                .push(TestFacts::of(node, self.content, name));
            let owner = Owner::Test(self.found.tests.len() - 1);
            self.open.push(Open { depth, owner });
            return true;
        }
        if let Some(name) = helpers::definition_name(node, self.content) {
            self.helper_bodies.push((name, Body::default()));
            let owner = Owner::Helper(self.helper_bodies.len() - 1);
            self.open.push(Open { depth, owner });
            return true;
        }
        if !self.open.is_empty() {
            self.record_body_evidence(node, depth);
        }
        true
    }

    fn record_body_evidence(&mut self, node: tree_sitter::Node<'_>, depth: usize) {
        let callee = helpers::callee_name(node, self.content);
        // Callee names are resolved against the file's helpers after the walk.
        let called = callee.map(|name| {
            (
                name.as_ptr() as usize - self.content.as_ptr() as usize,
                name.len(),
            )
        });
        let asserts = (self.tables.is_assertion)(node, self.content);
        let matcher = if asserts {
            self.tables
                .assertion_matcher
                .and_then(|recognize| recognize(node, self.content))
        } else {
            None
        };
        self.record_checks(asserts, matcher, called);
        if helpers::iterates(node, callee) {
            self.loops.push(depth);
        }
    }

    fn record_checks(
        &mut self,
        asserts: bool,
        matcher: Option<AssertionMatcher>,
        called: Option<(usize, usize)>,
    ) {
        if !asserts && called.is_none() {
            return;
        }
        for index in (0..self.open.len()).rev() {
            let owner = self.open[index].owner;
            let body_depth = self.open[index].depth;
            let in_loop = self.loops.last().is_some_and(|&start| start > body_depth);
            self.record_owner(owner, in_loop, asserts, matcher, called);
            if matches!(owner, Owner::Test(_)) {
                break;
            }
        }
    }

    fn record_owner(
        &mut self,
        owner: Owner,
        in_loop: bool,
        asserts: bool,
        matcher: Option<AssertionMatcher>,
        called: Option<(usize, usize)>,
    ) {
        let target = match owner {
            Owner::Test(index) => self.found.tests[index].body_mut(),
            Owner::Helper(index) => &mut self.helper_bodies[index].1,
        };
        if asserts {
            target.add_assertion(in_loop);
        }
        if let Some(matcher) = matcher {
            target.add_matcher(matcher);
        }
        if let Some((start, len)) = called {
            target.add_pending_call(start, len, in_loop);
        }
    }

    fn finish(mut self) -> Scan {
        let names: BTreeSet<String> = self
            .helper_bodies
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        for (_, body) in &mut self.helper_bodies {
            body.settle(self.content, &names);
        }
        for test in &mut self.found.tests {
            test.body_mut().settle(self.content, &names);
        }
        let mut helper_map = BTreeMap::new();
        for (name, body) in self.helper_bodies {
            helpers::remember(&mut helper_map, name, body);
        }
        for test in &mut self.found.tests {
            test.include_helpers(&helper_map);
        }
        self.found
    }
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
