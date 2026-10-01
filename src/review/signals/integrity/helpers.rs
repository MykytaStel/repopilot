//! What a test body checks beyond its own assertion statements.
//!
//! A test that moves its assertions into a helper defined in the same file
//! still checks them, so a test's count includes the same-file helpers it
//! calls, once per call and through further helpers. A test turned into a loop
//! over cases (a table-driven test) can check more with fewer assertion
//! statements, so the accounting does not compare it with its loop-free
//! version.

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

/// Assertions, loops and calls in one test or helper body, not counting
/// nested test cases; filled by the scan pass (`scan.rs`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Body {
    pub(super) assertions: usize,
    /// An assertion, or a call to a helper that asserts, sits inside a loop.
    pub(super) looped: bool,
    calls: BTreeMap<String, usize>,
    looped_calls: BTreeSet<String>,
    /// Calls seen during the scan, by byte range of the callee name and
    /// whether they ran in a loop; `settle` keeps those naming a helper.
    pending: Vec<(usize, usize, bool)>,
}

impl Body {
    pub(super) fn add_assertion(&mut self, in_loop: bool) {
        self.assertions += 1;
        self.looped |= in_loop;
    }

    fn add_call(&mut self, name: &str, in_loop: bool) {
        *self.calls.entry(name.to_string()).or_default() += 1;
        if in_loop {
            self.looped_calls.insert(name.to_string());
        }
    }

    /// Records a call by the byte range of its callee name in `content`.
    pub(super) fn add_pending_call(&mut self, start: usize, len: usize, in_loop: bool) {
        self.pending.push((start, len, in_loop));
    }

    /// Keeps the pending calls that name a same-file function.
    pub(super) fn settle(&mut self, content: &str, names: &BTreeSet<String>) {
        for (start, len, in_loop) in std::mem::take(&mut self.pending) {
            if let Some(name) = content.get(start..start + len)
                && names.contains(name)
            {
                self.add_call(name, in_loop);
            }
        }
    }
}

/// Whether a node runs its body once per element: a loop statement, or a
/// `forEach`-style call.
pub(super) fn iterates(node: Node<'_>, callee: Option<&str>) -> bool {
    LOOP_KINDS.contains(&node.kind()) || callee.is_some_and(|name| ITERATING_CALLS.contains(&name))
}

/// The last segment of a call's callee: `helper`, `self.helper`, `mod::helper`.
pub(super) fn callee_name<'a>(node: Node<'a>, content: &'a str) -> Option<&'a str> {
    if !CALL_KINDS.contains(&node.kind()) {
        return None;
    }
    // The callee is the first child of a call in every supported grammar;
    // `child_by_field_name` would search the field names on each call.
    let callee = node.child(0)?;
    let text = callee.utf8_text(content.as_bytes()).ok()?;
    let last = text.rsplit(['.', ':']).next()?.trim();
    (!last.is_empty()).then_some(last)
}

/// The name of a function, method, or function-valued variable defined here.
pub(super) fn definition_name(node: Node<'_>, content: &str) -> Option<String> {
    let defines_function = FUNCTION_KINDS.contains(&node.kind())
        || (node.kind() == "variable_declarator"
            // The value is the declarator's last named child, when present.
            && node
                .named_child(u32::try_from(node.named_child_count().saturating_sub(1)).unwrap_or(0))
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
