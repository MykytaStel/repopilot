//! Review-signal tables for Java: boundary node kinds, algorithmic node-kind
//! sets, and taint-lite tables (Servlet request sources; JDBC / process /
//! filesystem sinks).

use crate::review::signals::tables::{
    AlgorithmicKinds, BoundaryKinds, RemovedTables, ReviewTables,
};
use crate::review::signals::taint::sinks::{Sink, SinkKind};
use crate::review::signals::taint::tables::TaintTables;
use tree_sitter::Node;

/// Conservative, high-specificity taint tables. Sources are `HttpServletRequest`
/// reads; sinks mirror the JVM behavioral "added X" detectors (JDBC
/// `execute*`, `Runtime.exec`, `Files.write`). Kept deliberately narrow — Java
/// request handling is heavily framework-mediated, so only unambiguous idioms
/// participate, and everything ships at `preview`.
pub(super) static JAVA_TAINT: TaintTables = TaintTables {
    request_sources: &[
        "request.getParameter",
        "request.getParameterValues",
        "request.getParameterMap",
        "request.getHeader",
        "request.getHeaders",
        "request.getQueryString",
        "request.getCookies",
        "request.getInputStream",
        "request.getReader",
        "request.getPart",
    ],
    argv_sources: &[],
    // `request.getParameter("id")` is a method_invocation whose text carries
    // the idiom; that is the only node shape we match a Java source against.
    source_access_kinds: &["method_invocation"],
    // Coercion pruning is skipped for Java: `Integer.parseInt(...)` is a
    // method_invocation the shared sanitizer walker cannot key off, and a
    // request value coerced to an int rarely reaches a string-built sink.
    coercions: &[],
    coercion_call_kind: "method_invocation",
    assignment_kinds: &["variable_declarator", "assignment_expression"],
    is_flow_scope,
    is_augmenting,
    is_string_building,
    classify_sink,
};

fn is_flow_scope(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "method_declaration" | "constructor_declaration" | "lambda_expression"
    )
}

fn is_augmenting(node: Node<'_>) -> bool {
    node.kind() == "assignment_expression" && {
        let mut cursor = node.walk();
        node.children(&mut cursor).any(|child| {
            matches!(
                child.kind(),
                "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" | ">>>="
            )
        })
    }
}

fn is_string_building(node: Node<'_>, content: &str) -> bool {
    // `"..." + x` is a binary_expression; `String.format(...)` /
    // `str.formatted(...)` build a string from parts.
    node.kind() == "binary_expression"
        || (node.kind() == "method_invocation"
            && java_callee(node, content)
                .is_some_and(|callee| callee == "String.format" || callee.ends_with(".formatted")))
}

/// The `object.method` callee text of a Java `method_invocation`, with the
/// argument list stripped off. Java has no `function` field the shared
/// helpers rely on, so taint keys off the source text before `arguments`.
fn java_callee<'a>(node: Node<'a>, content: &'a str) -> Option<&'a str> {
    let args = node.child_by_field_name("arguments")?;
    content
        .get(node.start_byte()..args.start_byte())
        .map(str::trim)
        .filter(|callee| !callee.is_empty())
}

fn classify_sink<'a>(node: Node<'a>, content: &'a str) -> Option<Sink<'a>> {
    if node.kind() != "method_invocation" {
        return None;
    }
    let args = node.child_by_field_name("arguments")?;
    let callee = java_callee(node, content)?;

    let kind = if callee.ends_with(".executeQuery")
        || callee.ends_with(".executeUpdate")
        || callee.ends_with(".executeLargeUpdate")
        || callee.ends_with(".execute")
    {
        SinkKind::Sql
    } else if callee.ends_with(".exec") {
        // `Runtime.getRuntime().exec(...)`.
        SinkKind::Exec
    } else if callee == "Files.write" || callee == "Files.writeString" {
        SinkKind::FsWrite
    } else {
        return None;
    };
    Some(Sink { kind, args })
}

pub(super) static JAVA_REVIEW: ReviewTables = ReviewTables {
    boundary: Some(&BoundaryKinds {
        decorator_kinds: &["annotation"],
        import_kinds: &["import_declaration"],
    }),
    algorithmic: &AlgorithmicKinds {
        function_kinds: &["method_declaration", "constructor_declaration"],
        loop_kinds: &[
            "for_statement",
            "enhanced_for_statement",
            "while_statement",
            "do_statement",
        ],
        call_kinds: &["method_invocation"],
        control_flow_kinds: &[
            "if_statement",
            "for_statement",
            "enhanced_for_statement",
            "while_statement",
            "do_statement",
            "switch_statement",
            "try_statement",
        ],
        if_kinds: &["if_statement"],
    },
    removed: Some(&JAVA_REMOVED),
};

pub(super) static JAVA_REMOVED: RemovedTables = RemovedTables {
    extensions: &["java"],
    is_test_case: |node, content| {
        (node.kind() == "method_declaration" || node.kind() == "function_declaration")
            && node
                .utf8_text(content.as_bytes())
                .is_ok_and(|text| text.contains("@Test"))
    },
    is_error_handling: |node, _| node.kind() == "try_statement",
    auth_call_kinds: &["method_invocation", "call_expression"],
};
