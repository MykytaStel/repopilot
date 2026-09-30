//! Test-integrity recognizers for pytest and unittest: skip/xfail
//! decorators, runtime skip calls, a module-level `pytestmark` skip, `test_*`
//! cases, and `assert` / `self.assert*` / `pytest.raises` assertions.

use crate::review::signals::integrity::syntax::{
    all_arguments_literal, any_argument_string, bounded, compact_text, enclosing_function_name,
    is_literal,
};
use crate::review::signals::tables::{IntegrityTables, TestMarker, TestMarkerKind};
use tree_sitter::Node;

pub(super) static PYTHON_INTEGRITY: IntegrityTables = IntegrityTables {
    extensions: &["py"],
    applies_outside_test_files: false,
    test_marker,
    test_case,
    is_assertion,
};

const SKIP_DECORATORS: &[&str] = &[
    "pytest.mark.skip",
    "pytest.mark.skipif",
    "pytest.mark.xfail",
    "mark.skip",
    "mark.skipif",
    "mark.xfail",
    "unittest.skip",
    "unittest.skipIf",
    "unittest.skipUnless",
    "unittest.expectedFailure",
    "skip",
    "skipIf",
    "skipUnless",
    "expectedFailure",
];
/// Runtime skips. `pytest.importorskip` is left out: it guards an optional
/// dependency rather than disabling a test that used to run.
const SKIP_CALLS: &[&str] = &[
    "pytest.skip",
    "pytest.xfail",
    "self.skipTest",
    "SkipTest",
    "unittest.SkipTest",
];

fn test_marker<'a>(node: Node<'a>, content: &'a str) -> Option<TestMarker> {
    match node.kind() {
        "decorator" => decorator_marker(node, content),
        "call" => call_marker(node, content),
        "assignment" => module_mark(node, content),
        _ => None,
    }
}

fn decorator_marker(node: Node<'_>, content: &str) -> Option<TestMarker> {
    let expression = node.named_child(0)?;
    let (target, arguments) = if expression.kind() == "call" {
        (
            expression.child_by_field_name("function")?,
            expression.child_by_field_name("arguments"),
        )
    } else {
        (expression, None)
    };
    let name = compact_text(target, content)?;
    if !SKIP_DECORATORS.contains(&name.as_str()) {
        return None;
    }
    let test_name = node
        .parent()
        .filter(|parent| parent.kind() == "decorated_definition")
        .and_then(|parent| parent.child_by_field_name("definition"))
        .and_then(|definition| definition.child_by_field_name("name"))
        .and_then(|name| name.utf8_text(content.as_bytes()).ok())
        .map(bounded);
    Some(TestMarker {
        kind: TestMarkerKind::Skip,
        marker: format!("@{name}"),
        test_name,
        reason: arguments.and_then(|arguments| any_argument_string(arguments, content)),
        line: node.start_position().row + 1,
    })
}

fn call_marker(node: Node<'_>, content: &str) -> Option<TestMarker> {
    let name = compact_text(node.child_by_field_name("function")?, content)?;
    if !SKIP_CALLS.contains(&name.as_str()) {
        return None;
    }
    Some(TestMarker {
        kind: TestMarkerKind::Skip,
        marker: name,
        test_name: enclosing_function_name(node, content, &["function_definition"]),
        reason: node
            .child_by_field_name("arguments")
            .and_then(|arguments| any_argument_string(arguments, content)),
        line: node.start_position().row + 1,
    })
}

/// `pytestmark = pytest.mark.skip(...)` skips every test in the module.
fn module_mark(node: Node<'_>, content: &str) -> Option<TestMarker> {
    let left = compact_text(node.child_by_field_name("left")?, content)?;
    if left != "pytestmark" {
        return None;
    }
    let right = node.child_by_field_name("right")?;
    let right_text = compact_text(right, content)?;
    if !["mark.skip", "mark.xfail"]
        .iter()
        .any(|mark| right_text.contains(mark))
    {
        return None;
    }
    Some(TestMarker {
        kind: TestMarkerKind::Skip,
        marker: "pytestmark".to_string(),
        test_name: Some("<module>".to_string()),
        reason: None,
        line: node.start_position().row + 1,
    })
}

fn test_case<'a>(node: Node<'a>, content: &'a str) -> Option<String> {
    if node.kind() != "function_definition" {
        return None;
    }
    let name = node
        .child_by_field_name("name")?
        .utf8_text(content.as_bytes())
        .ok()?;
    if !name.starts_with("test") {
        return None;
    }
    let mut qualified = vec![name.to_string()];
    let mut current = node.parent();
    while let Some(candidate) = current {
        if candidate.kind() == "class_definition"
            && let Some(class) = candidate
                .child_by_field_name("name")
                .and_then(|class| class.utf8_text(content.as_bytes()).ok())
        {
            qualified.push(class.to_string());
        }
        current = candidate.parent();
    }
    qualified.reverse();
    Some(qualified.join("."))
}

fn is_assertion<'a>(node: Node<'a>, content: &'a str) -> bool {
    match node.kind() {
        // `assert True` / `assert 1` cannot fail.
        "assert_statement" => node
            .named_child(0)
            .is_some_and(|condition| !is_literal(condition)),
        "call" => {
            let Some(callee) = node
                .child_by_field_name("function")
                .and_then(|function| compact_text(function, content))
            else {
                return false;
            };
            let last = callee.rsplit('.').next().unwrap_or_default();
            let asserts = last.starts_with("assert")
                || matches!(callee.as_str(), "pytest.raises" | "pytest.warns");
            asserts
                && node
                    .child_by_field_name("arguments")
                    .is_none_or(|arguments| !all_arguments_literal(arguments))
        }
        _ => false,
    }
}
