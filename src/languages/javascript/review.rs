//! Taint tables for the JS dialect family. JS/TS (and their React variants)
//! share a grammar shape, so one table covers both frontends. Source idioms
//! target Express/Koa; sinks mirror the behavioral "added X" detectors.

use crate::review::signals::taint::sinks::{Sink, SinkKind, callee_text, receiver_method};
use crate::review::signals::taint::tables::TaintTables;
use tree_sitter::Node;

pub(super) static JS_FAMILY_TAINT: TaintTables = TaintTables {
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
        "ctx.query",
        "ctx.request.body",
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

fn classify_sink<'a>(node: Node<'a>, content: &'a str) -> Option<Sink<'a>> {
    let (callee, args) = callee_text(node, content)?;
    let kind = if callee.ends_with(".query")
        || callee == "query"
        || callee.ends_with(".execute")
        || callee == "execute"
    {
        SinkKind::Sql
    } else if callee_is_or_ends_with(
        callee,
        &[
            "exec",
            "spawn",
            "execFile",
            "fork",
            "execSync",
            "spawnSync",
            "execFileSync",
        ],
    ) {
        SinkKind::Exec
    } else if callee_is_or_ends_with(
        callee,
        &[
            "writeFile",
            "writeFileSync",
            "appendFile",
            "appendFileSync",
            "createWriteStream",
        ],
    ) {
        SinkKind::FsWrite
    } else if callee == "fetch"
        || callee == "axios"
        || receiver_method(
            callee,
            "axios",
            &[
                "request", "get", "post", "put", "patch", "delete", "head", "options",
            ],
        )
        || matches!(
            callee,
            "http.request" | "https.request" | "http.get" | "https.get"
        )
    {
        SinkKind::Network
    } else {
        return None;
    };
    Some(Sink { kind, args })
}

fn callee_is_or_ends_with(callee: &str, names: &[&str]) -> bool {
    names
        .iter()
        .any(|name| callee == *name || callee.ends_with(&format!(".{name}")))
}
