//! What a test body checks beyond its own assertion statements.
//!
//! A test that moves its assertions into a helper defined in the same file
//! still checks them, so a test's count includes the same-file helpers it
//! calls, once per call and through further helpers. A test turned into a loop
//! over cases (a table-driven test) can check more with fewer assertion
//! statements, so the accounting does not compare it with its loop-free
//! version.

use crate::review::signals::tables::IntegrityTables;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

const FUNCTION_KINDS: &[&str] = &[
    "function_declaration",
    "generator_function_declaration",
    "function_definition",
    "function_item",
    "method_definition",
    "method_declaration",
];
const FUNCTION_VALUES: &[&str] = &["arrow_function", "function_expression", "function"];
const CALL_KINDS: &[&str] = &["call_expression", "call"];
const LOOP_KINDS: &[&str] = &[
    "for_statement",
    "for_in_statement",
    "while_statement",
    "do_statement",
    "for_expression",
    "while_expression",
    "loop_expression",
];
/// Methods that run their callback once per element.
const ITERATING_CALLS: &[&str] = &["forEach", "for_each"];

/// Assertions, loops and calls in one body, not counting nested test cases.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Body {
    pub(super) assertions: usize,
    /// An assertion, or a call to a helper that asserts, sits inside a loop.
    pub(super) looped: bool,
    calls: BTreeMap<String, usize>,
    looped_calls: BTreeSet<String>,
}

pub(super) fn survey(node: Node<'_>, content: &str, tables: &IntegrityTables) -> Body {
    let mut body = Body::default();
    visit(node, content, tables, false, &mut body);
    body
}

fn visit(node: Node<'_>, content: &str, tables: &IntegrityTables, in_loop: bool, body: &mut Body) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if (tables.test_case)(child, content).is_some() {
            continue;
        }
        if (tables.is_assertion)(child, content) {
            body.assertions += 1;
            body.looped |= in_loop;
        }
        let callee = callee_name(child, content);
        if let Some(name) = &callee {
            *body.calls.entry(name.clone()).or_default() += 1;
            if in_loop {
                body.looped_calls.insert(name.clone());
            }
        }
        let iterates = LOOP_KINDS.contains(&child.kind())
            || callee.is_some_and(|name| ITERATING_CALLS.contains(&name.as_str()));
        visit(child, content, tables, in_loop || iterates, body);
    }
}

/// The last segment of a call's callee: `helper`, `self.helper`, `mod::helper`.
fn callee_name(node: Node<'_>, content: &str) -> Option<String> {
    if !CALL_KINDS.contains(&node.kind()) {
        return None;
    }
    let callee = node.child_by_field_name("function")?;
    let text = callee.utf8_text(content.as_bytes()).ok()?;
    let last = text.rsplit(['.', ':']).next()?.trim();
    (!last.is_empty()).then(|| last.to_string())
}

/// The name of a function, method, or function-valued variable defined here.
pub(super) fn definition_name(node: Node<'_>, content: &str) -> Option<String> {
    let defines_function = FUNCTION_KINDS.contains(&node.kind())
        || (node.kind() == "variable_declarator"
            && node
                .child_by_field_name("value")
                .is_some_and(|value| FUNCTION_VALUES.contains(&value.kind())));
    if !defines_function {
        return None;
    }
    let name = node.child_by_field_name("name")?;
    name.utf8_text(content.as_bytes()).ok().map(str::to_string)
}

/// Records a helper; a name defined twice keeps the larger body.
pub(super) fn remember(helpers: &mut BTreeMap<String, Body>, name: String, body: Body) {
    let entry = helpers.entry(name).or_default();
    if body.assertions >= entry.assertions {
        *entry = body;
    }
}

/// The body with the same-file helpers it reaches folded in.
pub(super) fn resolve(body: &Body, helpers: &BTreeMap<String, Body>) -> Body {
    let mut memo = BTreeMap::new();
    let mut resolved = body.clone();
    for (name, count) in &body.calls {
        let (assertions, looped) = reach(name, helpers, &mut memo, &mut BTreeSet::new());
        resolved.assertions = resolved
            .assertions
            .saturating_add(count.saturating_mul(assertions));
        resolved.looped |= looped || (assertions > 0 && body.looped_calls.contains(name));
    }
    resolved
}

/// Assertions a helper reaches, and whether any of them run in a loop.
fn reach(
    name: &str,
    helpers: &BTreeMap<String, Body>,
    memo: &mut BTreeMap<String, (usize, bool)>,
    active: &mut BTreeSet<String>,
) -> (usize, bool) {
    if let Some(known) = memo.get(name) {
        return *known;
    }
    let Some(helper) = helpers.get(name) else {
        return (0, false);
    };
    if !active.insert(name.to_string()) {
        return (0, false);
    }
    let resolved = helper.calls.iter().fold(
        (helper.assertions, helper.looped),
        |(sum, looped), (callee, count)| {
            let (assertions, nested) = reach(callee, helpers, memo, active);
            let in_loop = assertions > 0 && helper.looped_calls.contains(callee);
            (
                sum.saturating_add(count.saturating_mul(assertions)),
                looped || nested || in_loop,
            )
        },
    );
    active.remove(name);
    memo.insert(name.to_string(), resolved);
    resolved
}
