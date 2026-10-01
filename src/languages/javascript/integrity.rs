//! Test-integrity recognizers for the JS test runners: Jest, Vitest, Mocha,
//! Jasmine, and Playwright share the `it`/`test`/`describe` call shapes and
//! the `expect(...)`/`assert(...)` assertion shapes.

use crate::review::signals::integrity::syntax::{
    all_arguments_literal, any_argument_string, compact_text, first_argument_string, is_literal,
};
use crate::review::signals::tables::{IntegrityTables, TestMarker, TestMarkerKind};
use tree_sitter::Node;

pub(super) static JS_FAMILY_INTEGRITY: IntegrityTables = IntegrityTables {
    extensions: &["js", "mjs", "cjs", "ts", "mts", "cts", "tsx", "jsx"],
    applies_outside_test_files: false,
    test_marker,
    test_case,
    is_assertion,
    suppression: super::integrity_suppressions::suppression,
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
    let root = chain_root(node, content)?;
    if !(TEST_BASES.contains(&root)
        || FOCUSED_BASES.contains(&root)
        || EXCLUDED_BASES.contains(&root)
        || matches!(root, "this" | "ctx"))
    {
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
            && chain_root(candidate, content)
                .is_some_and(|root| TEST_BASES.contains(&root) || FOCUSED_BASES.contains(&root))
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

/// Bases that declare one test case, and chain segments a case may carry.
const CASE_BASES: &[&str] = &["it", "test", "specify", "fit", "xit", "xtest"];
const GROUP_BASES: &[&str] = &[
    "describe",
    "context",
    "suite",
    "fdescribe",
    "xdescribe",
    "fcontext",
    "xcontext",
];
const CASE_MODIFIERS: &[&str] = &[
    "only",
    "skip",
    "skipIf",
    "runIf",
    "todo",
    "fixme",
    "fail",
    "failing",
    "concurrent",
    "sequential",
    "serial",
    "parallel",
];

fn test_case<'a>(node: Node<'a>, content: &'a str) -> Option<String> {
    if node.kind() != "call_expression"
        || !chain_root(node, content).is_some_and(|root| CASE_BASES.contains(&root))
    {
        return None;
    }
    let callee = compact_text(node.child_by_field_name("function")?, content)?;
    let mut segments = callee.split('.');
    let base = segments.next()?;
    if !CASE_BASES.contains(&base) || !segments.all(|segment| CASE_MODIFIERS.contains(&segment)) {
        return None;
    }
    let name = first_argument_string(node, content)?;
    let mut qualified = vec![name];
    let mut current = node.parent();
    while let Some(candidate) = current {
        if candidate.kind() == "call_expression"
            && let Some(group) = group_name(candidate, content)
        {
            qualified.push(group);
        }
        current = candidate.parent();
    }
    qualified.reverse();
    Some(qualified.join(" > "))
}

/// The title of a `describe`-like group call (`describe`, `test.describe`).
fn group_name(node: Node<'_>, content: &str) -> Option<String> {
    if !chain_root(node, content).is_some_and(|root| GROUP_BASES.contains(&root) || root == "test")
    {
        return None;
    }
    let callee = compact_text(node.child_by_field_name("function")?, content)?;
    let mut segments = callee.split('.');
    let base = segments.next()?;
    let is_group =
        GROUP_BASES.contains(&base) || (base == "test" && segments.next() == Some("describe"));
    is_group
        .then(|| first_argument_string(node, content))
        .flatten()
}

fn is_assertion<'a>(node: Node<'a>, content: &'a str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    let may_assert = chain_root(node, content)
        .is_some_and(|root| matches!(root, "expect" | "assert"))
        || function
            .utf8_text(content.as_bytes())
            .is_ok_and(|text| text.contains("should"));
    if !may_assert {
        return false;
    }
    let Some(callee) = compact_text(function, content) else {
        return false;
    };
    let arguments = node.child_by_field_name("arguments");
    if callee.starts_with("expect(") && function.kind() == "member_expression" {
        // `expect(x).toBe(y)`: the matcher call. A literal subject cannot
        // observe the code under test (`expect(true).toBe(true)`).
        return expect_subject(function).is_none_or(|subject| !is_literal(subject));
    }
    let base = callee.split('.').next().unwrap_or_default();
    if base == "assert" || callee.contains(".should.") {
        return arguments.is_none_or(|arguments| !all_arguments_literal(arguments));
    }
    false
}

/// The identifier a call chain starts from: `it` in `it.only(...)`, `expect` in
/// `expect(x).toBe(y)`, `describe` in `describe.each(t)(...)`. Recognizers check
/// it before copying the callee text, which most calls never need.
fn chain_root<'a>(call: Node<'a>, content: &'a str) -> Option<&'a str> {
    let mut current = call.child_by_field_name("function")?;
    loop {
        match current.kind() {
            "identifier" | "this" => return current.utf8_text(content.as_bytes()).ok(),
            "member_expression" => current = current.child_by_field_name("object")?,
            "call_expression" => current = current.child_by_field_name("function")?,
            _ => return None,
        }
    }
}

/// The single argument of the `expect(...)` call at the root of a matcher chain.
fn expect_subject(function: Node<'_>) -> Option<Node<'_>> {
    let mut current = function;
    loop {
        match current.kind() {
            "member_expression" => current = current.child_by_field_name("object")?,
            "call_expression" => {
                let callee = current.child_by_field_name("function")?;
                if callee.kind() == "identifier" {
                    return current.child_by_field_name("arguments")?.named_child(0);
                }
                current = callee;
            }
            _ => return None,
        }
    }
}
