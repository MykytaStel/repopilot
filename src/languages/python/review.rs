//! Taint tables for Python. Source idioms target Flask/Django/FastAPI; sinks
//! mirror the behavioral "added X" detectors.

use crate::review::signals::tables::{
    AlgorithmicKinds, BoundaryKinds, RemovedTables, ReviewTables,
};
use crate::review::signals::taint::ast::{callee_ends_with, has_descendant_kind};
use crate::review::signals::taint::sinks::{Sink, SinkKind, callee_text, receiver_method};
use crate::review::signals::taint::tables::TaintTables;
use tree_sitter::Node;

pub(super) static PYTHON_TAINT: TaintTables = TaintTables {
    request_sources: &[
        "request.args",
        "request.form",
        "request.json",
        "request.values",
        "request.data",
        "request.files",
        "request.GET",
        "request.POST",
        "request.query_params",
        "request.path_params",
        "request.body",
        "request.cookies",
        "request.headers",
    ],
    argv_sources: &["sys.argv"],
    source_access_kinds: &["attribute"],
    coercions: &["int", "float", "bool"],
    coercion_call_kind: "call",
    assignment_kinds: &["assignment", "augmented_assignment"],
    is_flow_scope,
    is_augmenting: |node| node.kind() == "augmented_assignment",
    is_string_building,
    classify_sink,
};

fn is_flow_scope(node: Node<'_>) -> bool {
    matches!(node.kind(), "function_definition" | "lambda")
}

fn is_string_building(node: Node<'_>, content: &str) -> bool {
    node.kind() == "binary_operator"
        || (node.kind() == "string" && has_descendant_kind(node, "interpolation"))
        || (node.kind() == "call" && callee_ends_with(node, content, ".format"))
}

fn classify_sink<'a>(node: Node<'a>, content: &'a str) -> Option<Sink<'a>> {
    let (callee, args) = callee_text(node, content)?;
    let kind = if callee.ends_with(".execute")
        || callee.ends_with(".executemany")
        || callee.ends_with(".query")
        || callee.ends_with(".raw")
    {
        SinkKind::Sql
    } else if receiver_method(
        callee,
        "subprocess",
        &[
            "run",
            "Popen",
            "call",
            "check_call",
            "check_output",
            "getoutput",
            "getstatusoutput",
        ],
    ) || callee == "os.system"
        || callee == "os.popen"
        || callee == "eval"
        || callee == "exec"
    {
        SinkKind::Exec
    } else if (callee == "open" && python_open_is_write(args, content))
        || callee.ends_with(".write_text")
        || callee.ends_with(".write_bytes")
    {
        SinkKind::FsWrite
    } else if receiver_method(
        callee,
        "requests",
        &[
            "request", "get", "post", "put", "patch", "delete", "head", "options",
        ],
    ) || receiver_method(
        callee,
        "httpx",
        &[
            "request", "get", "post", "put", "patch", "delete", "head", "options",
        ],
    ) || callee == "urllib.request.urlopen"
        || callee == "urlopen"
    {
        SinkKind::Network
    } else {
        return None;
    };
    Some(Sink { kind, args })
}

fn python_open_is_write(args: Node<'_>, content: &str) -> bool {
    let mut cursor = args.walk();
    for (index, arg) in args.named_children(&mut cursor).enumerate() {
        if index == 0 {
            continue;
        }

        let mode = if arg.kind() == "keyword_argument" {
            let Some(name) = arg.child_by_field_name("name") else {
                continue;
            };
            if name.utf8_text(content.as_bytes()).ok() != Some("mode") {
                continue;
            }
            arg.child_by_field_name("value")
        } else if index == 1 {
            Some(arg)
        } else {
            None
        };

        if mode.is_some_and(|mode| {
            let text = mode.utf8_text(content.as_bytes()).unwrap_or("");
            quoted_value(text).is_some_and(|value| {
                value
                    .chars()
                    .any(|character| matches!(character, 'w' | 'a' | 'x' | '+'))
            })
        }) {
            return true;
        }
    }
    false
}

fn quoted_value(text: &str) -> Option<&str> {
    let text = text.trim();
    let quote_index = text.find(['\'', '"'])?;
    let quote = text.as_bytes()[quote_index];
    let value = &text[quote_index + 1..];
    let end = value.as_bytes().iter().position(|byte| *byte == quote)?;
    Some(&value[..end])
}

pub(super) static PYTHON_REVIEW: ReviewTables = ReviewTables {
    boundary: Some(&BoundaryKinds {
        decorator_kinds: &["decorator"],
        import_kinds: &["import_statement", "import_from_statement"],
    }),
    algorithmic: &AlgorithmicKinds {
        function_kinds: &["function_definition"],
        loop_kinds: &["for_statement", "while_statement"],
        call_kinds: &["call"],
        control_flow_kinds: &[
            "if_statement",
            "for_statement",
            "while_statement",
            "match_statement",
            "try_statement",
        ],
        if_kinds: &["if_statement"],
    },
    removed: Some(&PYTHON_REMOVED),
};

pub(super) static PYTHON_REMOVED: RemovedTables = RemovedTables {
    extensions: &["py"],
    is_test_case: |node, content| {
        node.kind() == "function_definition"
            && node
                .child_by_field_name("name")
                .and_then(|name| name.utf8_text(content.as_bytes()).ok())
                .is_some_and(|name| name.starts_with("test_"))
    },
    is_error_handling: |node, _| node.kind() == "try_statement",
    auth_call_kinds: &["call"],
};
