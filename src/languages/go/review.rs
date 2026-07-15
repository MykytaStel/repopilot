//! Taint tables for Go. Source idioms target net/http and Gin; sinks mirror
//! the behavioral "added X" detectors.

use crate::review::signals::tables::{
    AlgorithmicKinds, BoundaryKinds, RemovedTables, ReviewTables,
};
use crate::review::signals::taint::ast::callee_starts_with;
use crate::review::signals::taint::sinks::{Sink, SinkKind, callee_text};
use crate::review::signals::taint::tables::TaintTables;
use tree_sitter::Node;

pub(super) static GO_TAINT: TaintTables = TaintTables {
    request_sources: &[
        "r.URL.Query",
        "r.FormValue",
        "r.PostFormValue",
        "r.PostForm",
        "r.Form",
        "r.Body",
        "r.Header.Get",
        "c.Query",
        "c.Param",
        "c.PostForm",
        "ctx.Query",
    ],
    argv_sources: &["os.Args"],
    source_access_kinds: &["selector_expression"],
    coercions: &[
        "strconv.Atoi",
        "strconv.ParseInt",
        "strconv.ParseFloat",
        "strconv.ParseBool",
    ],
    coercion_call_kind: "call_expression",
    assignment_kinds: &["short_var_declaration", "assignment_statement"],
    is_flow_scope,
    is_augmenting,
    is_string_building,
    classify_sink,
};

fn is_flow_scope(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "function_declaration" | "method_declaration" | "func_literal"
    )
}

fn is_augmenting(node: Node<'_>) -> bool {
    node.kind() == "assignment_statement" && {
        let mut cursor = node.walk();
        node.children(&mut cursor).any(|child| {
            matches!(
                child.kind(),
                "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" | "&^="
            )
        })
    }
}

fn is_string_building(node: Node<'_>, content: &str) -> bool {
    node.kind() == "binary_expression"
        || (node.kind() == "call_expression" && callee_starts_with(node, content, "fmt.Sprint"))
}

fn classify_sink<'a>(node: Node<'a>, content: &'a str) -> Option<Sink<'a>> {
    let (callee, args) = callee_text(node, content)?;
    let kind = if callee.ends_with(".Query")
        || callee.ends_with(".QueryRow")
        || callee.ends_with(".QueryContext")
        || callee.ends_with(".Exec")
        || callee.ends_with(".ExecContext")
    {
        SinkKind::Sql
    } else if matches!(callee, "exec.Command" | "exec.CommandContext") {
        SinkKind::Exec
    } else if matches!(
        callee,
        "os.WriteFile" | "os.Create" | "os.CreateTemp" | "ioutil.WriteFile"
    ) || (callee == "os.OpenFile" && go_open_file_is_write(args, content))
    {
        SinkKind::FsWrite
    } else if matches!(
        callee,
        "http.Get"
            | "http.Post"
            | "http.PostForm"
            | "http.NewRequest"
            | "http.NewRequestWithContext"
            | "net.Dial"
            | "net.DialTimeout"
    ) || callee.ends_with(".Do")
        || callee.ends_with(".DialContext")
    {
        SinkKind::Network
    } else {
        return None;
    };
    Some(Sink { kind, args })
}

fn go_open_file_is_write(args: Node<'_>, content: &str) -> bool {
    let mut cursor = args.walk();
    let Some(flags) = args.named_children(&mut cursor).nth(1) else {
        return false;
    };
    let text = flags.utf8_text(content.as_bytes()).unwrap_or("");
    [
        "os.O_WRONLY",
        "os.O_RDWR",
        "os.O_APPEND",
        "os.O_CREATE",
        "os.O_TRUNC",
    ]
    .iter()
    .any(|flag| text.contains(flag))
}

pub(super) static GO_REVIEW: ReviewTables = ReviewTables {
    boundary: Some(&BoundaryKinds {
        decorator_kinds: &[],
        import_kinds: &["import_spec", "import_declaration"],
    }),
    algorithmic: &AlgorithmicKinds {
        function_kinds: &["function_declaration", "method_declaration"],
        loop_kinds: &["for_statement"],
        call_kinds: &["call_expression"],
        control_flow_kinds: &[
            "if_statement",
            "for_statement",
            "expression_switch_statement",
            "type_switch_statement",
            "select_statement",
        ],
        if_kinds: &["if_statement"],
    },
    removed: Some(&GO_REMOVED),
};

pub(super) static GO_REMOVED: RemovedTables = RemovedTables {
    extensions: &["go"],
    is_test_case: |node, content| {
        node.kind() == "function_declaration"
            && node
                .child_by_field_name("name")
                .and_then(|name| name.utf8_text(content.as_bytes()).ok())
                .is_some_and(|name| name.starts_with("Test"))
    },
    // Inspect only the `if` condition, not the whole statement body, so a
    // nested `if err != nil` does not inflate the enclosing block's count.
    is_error_handling: |node, content| {
        node.kind() == "if_statement"
            && node
                .child_by_field_name("condition")
                .and_then(|cond| cond.utf8_text(content.as_bytes()).ok())
                .is_some_and(|cond| cond.contains("err") && cond.contains("nil"))
    },
    auth_call_kinds: &["call_expression"],
};
