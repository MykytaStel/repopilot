use super::sinks::classify_sink;
use crate::review::signals::tables::{AlgorithmicKinds, BoundaryKinds, ReviewTables};
use crate::review::signals::taint::tables::TaintTables;
use tree_sitter::Node;

pub static JS_FAMILY_TAINT: TaintTables = TaintTables {
    request_sources: &[
        "req.query",
        "req.params",
        "req.body",
        "req.headers",
        "req.cookies",
        "req.url",
        "request.query",
        "request.params",
        "request.body",
        "request.headers",
        "request.cookies",
        "request.url",
        "request.nextUrl.searchParams",
        "req.nextUrl.searchParams",
        "request.json",
        "req.json",
        "request.formData",
        "req.formData",
        "request.raw.url",
        "req.raw.url",
        "request.ip",
        "req.ip",
        "request.ips",
        "req.ips",
        "request.hostname",
        "req.hostname",
        "request.protocol",
        "req.protocol",
        "ctx.query",
        "ctx.request.body",
        "c.req.query",
        "c.req.queries",
        "c.req.param",
        "c.req.header",
        "c.req.json",
        "c.req.parseBody",
        "c.req.text",
        "c.req.arrayBuffer",
        "c.req.raw",
        "c.req.url",
    ],
    argv_sources: &["process.argv"],
    source_access_kinds: &["member_expression"],
    coercions: &["Number", "parseInt", "parseFloat", "BigInt", "Boolean"],
    coercion_call_kind: "call_expression",
    assignment_kinds: &["variable_declarator", "assignment_expression"],
    is_flow_scope,
    // tree-sitter-javascript models `x += …` as a distinct
    // `augmented_assignment_expression` node that assignment collection does
    // not pick up, so anything collected is a plain `=`.
    is_augmenting: |_| false,
    is_string_building,
    classify_sink,
};

fn is_flow_scope(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "function_declaration"
            | "generator_function_declaration"
            | "method_definition"
            | "function_expression"
            | "generator_function"
            | "arrow_function"
    )
}

fn is_string_building(node: Node<'_>, _content: &str) -> bool {
    matches!(node.kind(), "template_string" | "binary_expression")
}

pub static JS_FAMILY_REVIEW: ReviewTables = ReviewTables {
    boundary: Some(&BoundaryKinds {
        decorator_kinds: &["decorator"],
        import_kinds: &["import_statement", "export_statement", "call_expression"],
    }),
    algorithmic: &AlgorithmicKinds {
        function_kinds: &[
            "function_declaration",
            "generator_function_declaration",
            "method_definition",
        ],
        loop_kinds: &[
            "for_statement",
            "for_in_statement",
            "for_of_statement",
            "while_statement",
            "do_statement",
        ],
        call_kinds: &["call_expression"],
        control_flow_kinds: &[
            "if_statement",
            "for_statement",
            "for_in_statement",
            "for_of_statement",
            "while_statement",
            "do_statement",
            "switch_statement",
            "try_statement",
        ],
        if_kinds: &["if_statement"],
    },
    removed: Some(&super::super::removed::JS_FAMILY_REMOVED),
    integrity: Some(&super::super::integrity::JS_FAMILY_INTEGRITY),
};
