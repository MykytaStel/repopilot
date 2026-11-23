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
    let kind = sink_kind(callee, args, content)?;
    let mut sink = Sink::new(kind, args);
    if kind == SinkKind::Exec {
        sink.safe_exec_command = safe_exec_command(callee, args, content);
    }
    Some(sink)
}

fn sink_kind(callee: &str, args: Node<'_>, content: &str) -> Option<SinkKind> {
    if callee.ends_with(".execute")
        || callee.ends_with(".executemany")
        || callee.ends_with(".query")
        || callee.ends_with(".raw")
    {
        Some(SinkKind::Sql)
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
        Some(SinkKind::Exec)
    } else if (callee == "open" && python_open_is_write(args, content))
        || callee.ends_with(".write_text")
        || callee.ends_with(".write_bytes")
    {
        Some(SinkKind::FsWrite)
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
        Some(SinkKind::Network)
    } else {
        None
    }
}

fn safe_exec_command<'a>(callee: &str, args: Node<'a>, content: &str) -> Option<Node<'a>> {
    if ![
        "subprocess.run",
        "subprocess.Popen",
        "subprocess.call",
        "subprocess.check_call",
        "subprocess.check_output",
    ]
    .contains(&callee)
    {
        return None;
    }
    let vector = command_vector(args, content)?;
    safe_vector_command(vector, content)
}

fn command_vector<'a>(args: Node<'a>, content: &str) -> Option<Node<'a>> {
    let mut cursor = args.walk();
    let arguments = args.named_children(&mut cursor).collect::<Vec<_>>();
    let mut vector = None;
    for argument in arguments {
        if argument.kind() == "dictionary_splat" {
            return None;
        }
        if keyword_name(argument, content) == Some("shell") && !shell_disabled(argument, content) {
            return None;
        }
        if keyword_name(argument, content) == Some("args") {
            vector = argument.child_by_field_name("value");
        } else if vector.is_none() && argument.kind() != "keyword_argument" {
            vector = Some(argument);
        }
    }
    vector
}

fn keyword_name<'a>(argument: Node<'_>, content: &'a str) -> Option<&'a str> {
    (argument.kind() == "keyword_argument")
        .then(|| argument.child_by_field_name("name"))
        .flatten()?
        .utf8_text(content.as_bytes())
        .ok()
}

fn shell_disabled(argument: Node<'_>, content: &str) -> bool {
    argument
        .child_by_field_name("value")
        .and_then(|value| value.utf8_text(content.as_bytes()).ok())
        == Some("False")
}

fn safe_vector_command<'a>(vector: Node<'a>, content: &str) -> Option<Node<'a>> {
    if !matches!(vector.kind(), "list" | "tuple") {
        return None;
    }
    let mut cursor = vector.walk();
    let command = vector.named_children(&mut cursor).next()?;
    if command.kind() != "string" {
        return None;
    }
    let command_text = command.utf8_text(content.as_bytes()).ok()?;
    let shell = super::super::shell::is_shell_executable(command_text);
    let mut cursor = vector.walk();
    let nested_shell_flag = super::super::shell::has_shell_command_flag(
        vector
            .named_children(&mut cursor)
            .skip(1)
            .filter_map(|node| node.utf8_text(content.as_bytes()).ok()),
    );
    (!shell || !nested_shell_flag).then_some(command)
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
    integrity: Some(&super::integrity::PYTHON_INTEGRITY),
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
