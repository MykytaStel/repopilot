//! Skip and focus markers for the JS test runners: Jest, Vitest, Mocha,
//! Jasmine, and Playwright share the `it`/`test`/`describe` call shapes.

use crate::review::signals::integrity::syntax::{
    any_argument_string, compact_text, first_argument_string,
};
use crate::review::signals::tables::{IntegrityTables, TestMarker, TestMarkerKind};
use tree_sitter::Node;

pub(super) static JS_FAMILY_INTEGRITY: IntegrityTables = IntegrityTables {
    extensions: &["js", "mjs", "cjs", "ts", "mts", "cts", "tsx", "jsx"],
    applies_outside_test_files: false,
    test_marker,
};

const TEST_BASES: &[&str] = &["it", "test", "describe", "context", "suite", "specify"];
const FOCUSED_BASES: &[&str] = &["fit", "fdescribe", "fcontext"];
const EXCLUDED_BASES: &[&str] = &["xit", "xtest", "xdescribe", "xcontext", "xspecify"];
/// Chain segments that change how a test runs but not whether it runs.
const TRANSPARENT: &[&str] = &[
    "each",
    "concurrent",
    "sequential",
    "serial",
    "parallel",
    "describe",
];
const SKIP_MODIFIERS: &[&str] = &["skip", "skipIf", "todo", "fixme", "fail"];
/// Runtime skips called from inside a test body (Mocha `this`, Vitest `ctx`).
const RUNTIME_SKIPS: &[&str] = &["this.skip", "ctx.skip"];

fn test_marker<'a>(node: Node<'a>, content: &'a str) -> Option<TestMarker> {
    if node.kind() != "call_expression" {
        return None;
    }
    let callee = node.child_by_field_name("function")?;
    if !matches!(callee.kind(), "identifier" | "member_expression") {
        return None;
    }
    let marker = compact_text(callee, content)?;
    let kind = classify(&marker)?;
    let named_here = first_argument_string(node, content);
    let reason = named_here
        .is_none()
        .then(|| {
            node.child_by_field_name("arguments")
                .and_then(|arguments| any_argument_string(arguments, content))
        })
        .flatten();
    let test_name = named_here.or_else(|| enclosing_test_name(node, content));
    Some(TestMarker {
        kind,
        marker,
        test_name,
        reason,
        line: node.start_position().row + 1,
    })
}

fn classify(callee: &str) -> Option<TestMarkerKind> {
    if RUNTIME_SKIPS.contains(&callee) {
        return Some(TestMarkerKind::Skip);
    }
    let mut segments = callee.split('.');
    let base = segments.next()?;
    let modifiers: Vec<&str> = segments
        .filter(|segment| !TRANSPARENT.contains(segment))
        .collect();
    if FOCUSED_BASES.contains(&base) && modifiers.is_empty() {
        return Some(TestMarkerKind::Focus);
    }
    if EXCLUDED_BASES.contains(&base) && modifiers.is_empty() {
        return Some(TestMarkerKind::Skip);
    }
    if !TEST_BASES.contains(&base) {
        return None;
    }
    if modifiers.contains(&"only") {
        Some(TestMarkerKind::Focus)
    } else if modifiers
        .iter()
        .any(|modifier| SKIP_MODIFIERS.contains(modifier))
    {
        Some(TestMarkerKind::Skip)
    } else {
        None
    }
}

/// The title of the nearest enclosing `it`/`test`/`describe` call.
fn enclosing_test_name(node: Node<'_>, content: &str) -> Option<String> {
    let mut current = node.parent();
    while let Some(candidate) = current {
        if candidate.kind() == "call_expression"
            && let Some(callee) = candidate.child_by_field_name("function")
            && let Some(text) = compact_text(callee, content)
            && text
                .split('.')
                .next()
                .is_some_and(|base| TEST_BASES.contains(&base) || FOCUSED_BASES.contains(&base))
            && let Some(name) = first_argument_string(candidate, content)
        {
            return Some(name);
        }
        current = candidate.parent();
    }
    None
}
