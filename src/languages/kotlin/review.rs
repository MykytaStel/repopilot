//! Review-signal tables for Kotlin: boundary node kinds and algorithmic
//! node-kind sets.

use crate::review::signals::tables::{
    AlgorithmicKinds, BoundaryKinds, RemovedTables, ReviewTables,
};
use crate::review::signals::taint::sinks::{Sink, SinkKind};
use crate::review::signals::taint::tables::TaintTables;
use tree_sitter::Node;

/// Conservative, high-specificity taint tables for Kotlin (Ktor, Android,
/// Servlet request sources → Exec / SQL / FsWrite sinks).
pub(super) static KOTLIN_TAINT: TaintTables = TaintTables {
    request_sources: &[
        "call.receive",
        "call.receiveText",
        "call.receiveParameters",
        "call.parameters",
        "call.request.queryParameters",
        "call.request.headers",
        "call.request.cookies",
        "request.getParameter",
        "request.getHeader",
        "intent.getStringExtra",
        "savedStateHandle.get",
    ],
    argv_sources: &[],
    source_access_kinds: &["call_expression", "navigation_expression"],
    coercions: &[],
    coercion_call_kind: "call_expression",
    assignment_kinds: &["property_delegate", "variable_declaration", "assignment"],
    is_flow_scope,
    is_augmenting,
    is_string_building,
    classify_sink,
};

fn is_flow_scope(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "function_declaration"
            | "primary_constructor"
            | "secondary_constructor"
            | "class_initializer"
            | "property_accessor"
            | "lambda_literal"
    )
}

fn is_augmenting(node: Node<'_>) -> bool {
    node.kind() == "assignment" && {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .any(|child| matches!(child.kind(), "+=" | "-=" | "*=" | "/=" | "%="))
    }
}

fn is_string_building(node: Node<'_>, content: &str) -> bool {
    matches!(
        node.kind(),
        "additive_expression"
            | "string_literal"
            | "line_string_expression"
            | "multi_line_string_expression"
    ) || (node.kind() == "call_expression"
        && kotlin_callee(node, content)
            .is_some_and(|callee| callee.ends_with(".format") || callee == "String.format"))
}

fn kotlin_callee<'a>(node: Node<'a>, content: &'a str) -> Option<&'a str> {
    let args = node.child_by_field_name("value_arguments").or_else(|| {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .find(|c| c.kind() == "value_arguments")
    })?;
    let raw = content
        .get(node.start_byte()..args.start_byte())
        .map(str::trim)
        .filter(|callee| !callee.is_empty())?;
    let callee = match raw.find('<') {
        Some(idx) => raw[..idx].trim(),
        None => raw,
    };
    Some(callee)
}

fn classify_sink<'a>(node: Node<'a>, content: &'a str) -> Option<Sink<'a>> {
    if node.kind() != "call_expression" {
        return None;
    }
    let args = node.child_by_field_name("value_arguments").or_else(|| {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .find(|c| c.kind() == "value_arguments")
    })?;
    let callee = kotlin_callee(node, content)?;

    let kind = if callee.ends_with(".executeQuery")
        || callee.ends_with(".executeUpdate")
        || callee.ends_with(".executeLargeUpdate")
        || callee.ends_with(".execute")
        || callee.ends_with(".rawQuery")
    {
        SinkKind::Sql
    } else if callee.ends_with(".exec") || callee.contains("ProcessBuilder") {
        SinkKind::Exec
    } else if callee.ends_with(".writeText")
        || callee.ends_with(".writeBytes")
        || callee == "Files.write"
    {
        SinkKind::FsWrite
    } else {
        return None;
    };
    Some(Sink { kind, args })
}

pub(super) static KOTLIN_REVIEW: ReviewTables = ReviewTables {
    boundary: Some(&BoundaryKinds {
        decorator_kinds: &["annotation"],
        import_kinds: &["import_declaration"],
    }),
    algorithmic: &AlgorithmicKinds {
        function_kinds: &["function_declaration"],
        loop_kinds: &["for_statement", "while_statement", "do_while_statement"],
        call_kinds: &["call_expression"],
        control_flow_kinds: &[
            "if_expression",
            "when_expression",
            "for_statement",
            "while_statement",
            "do_while_statement",
            "try_expression",
        ],
        if_kinds: &["if_expression"],
    },
    removed: Some(&KOTLIN_REMOVED),
};

pub(super) static KOTLIN_REMOVED: RemovedTables = RemovedTables {
    extensions: &["kt", "kts"],
    is_test_case: |node, content| {
        (node.kind() == "method_declaration" || node.kind() == "function_declaration")
            && node
                .utf8_text(content.as_bytes())
                .is_ok_and(|text| text.contains("@Test"))
    },
    is_error_handling: |node, _| node.kind() == "try_expression",
    auth_call_kinds: &["method_invocation", "call_expression"],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::parse::{ParseLanguage, parse};

    #[test]
    fn classifies_kotlin_sinks_and_sources() {
        let code = r#"
            fun handleRequest(call: ApplicationCall) {
                val input = call.receiveText()
                File("output.txt").writeText(input)
                Runtime.getRuntime().exec(input)
            }
        "#;
        let tree = parse(code, ParseLanguage::Kotlin).unwrap();
        let root = tree.root_node();

        assert!(KOTLIN_TAINT.request_sources.contains(&"call.receiveText"));

        let mut sinks = Vec::new();
        fn find_calls<'a>(node: Node<'a>, content: &'a str, sinks: &mut Vec<SinkKind>) {
            if let Some(sink) = classify_sink(node, content) {
                sinks.push(sink.kind);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                find_calls(child, content, sinks);
            }
        }
        find_calls(root, code, &mut sinks);

        assert_eq!(sinks, vec![SinkKind::FsWrite, SinkKind::Exec]);
    }

    #[test]
    fn classifies_kotlin_generic_calls_and_sinks() {
        let code = r#"
            fun process(call: ApplicationCall) {
                val data = call.receive<String>()
                File("output.txt").writeText(data)
            }
        "#;
        let tree = parse(code, ParseLanguage::Kotlin).unwrap();
        let root = tree.root_node();

        let mut sinks = Vec::new();
        fn find_calls<'a>(node: Node<'a>, content: &'a str, sinks: &mut Vec<SinkKind>) {
            if let Some(sink) = classify_sink(node, content) {
                sinks.push(sink.kind);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                find_calls(child, content, sinks);
            }
        }
        find_calls(root, code, &mut sinks);

        assert_eq!(sinks, vec![SinkKind::FsWrite]);
    }
}
