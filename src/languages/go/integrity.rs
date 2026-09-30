//! Skip markers for Go's `testing` package: `t.Skip`, `t.Skipf`, and
//! `t.SkipNow` on a test, benchmark, fuzz, or suite handle.

use crate::review::signals::integrity::syntax::{
    any_argument_string, compact_text, enclosing_function_name,
};
use crate::review::signals::tables::{IntegrityTables, TestMarker, TestMarkerKind};
use tree_sitter::Node;

pub(super) static GO_INTEGRITY: IntegrityTables = IntegrityTables {
    extensions: &["go"],
    applies_outside_test_files: false,
    test_marker,
};

const SKIP_METHODS: &[&str] = &["Skip", "Skipf", "SkipNow"];

fn test_marker<'a>(node: Node<'a>, content: &'a str) -> Option<TestMarker> {
    if node.kind() != "call_expression" {
        return None;
    }
    let function = node.child_by_field_name("function")?;
    if function.kind() != "selector_expression" {
        return None;
    }
    let method = function
        .child_by_field_name("field")?
        .utf8_text(content.as_bytes())
        .ok()?;
    if !SKIP_METHODS.contains(&method) {
        return None;
    }
    // `t.Skip()` on a `testing` handle, or `s.T().Skip()` on a testify suite.
    let operand = function.child_by_field_name("operand")?;
    let receiver = compact_text(operand, content)?;
    let is_handle = match operand.kind() {
        "identifier" => is_testing_parameter(node, content, &receiver),
        "call_expression" => receiver.ends_with(".T()") || receiver.ends_with(".B()"),
        _ => false,
    };
    if !is_handle {
        return None;
    }
    Some(TestMarker {
        kind: TestMarkerKind::Skip,
        marker: format!("{receiver}.{method}"),
        test_name: enclosing_function_name(
            node,
            content,
            &["function_declaration", "method_declaration"],
        ),
        reason: node
            .child_by_field_name("arguments")
            .and_then(|arguments| any_argument_string(arguments, content)),
        line: node.start_position().row + 1,
    })
}

/// Whether `receiver` is declared as a `*testing.T`/`B`/`F` or `testing.TB`
/// parameter of an enclosing function or closure — so `reader.Skip(4)` in a
/// test file is not mistaken for a skipped test.
fn is_testing_parameter(node: Node<'_>, content: &str, receiver: &str) -> bool {
    let mut current = node.parent();
    while let Some(candidate) = current {
        if matches!(
            candidate.kind(),
            "function_declaration" | "method_declaration" | "func_literal"
        ) && let Some(parameters) = candidate
            .child_by_field_name("parameters")
            .and_then(|parameters| compact_text(parameters, content))
        {
            let declared = ["(", ","].iter().any(|open| {
                parameters.contains(&format!("{open}{receiver}*testing."))
                    || parameters.contains(&format!("{open}{receiver}testing.TB"))
            });
            if declared {
                return true;
            }
        }
        current = candidate.parent();
    }
    false
}
