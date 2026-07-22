//! Taint tables for C#. Sources are ASP.NET request reads; sinks mirror the
//! C# behavioral "added X" detectors (`Process.Start`, SqlCommand
//! `Execute*`, `File.WriteAll*`). Conservative and high-specificity —
//! ASP.NET request handling is heavily framework-mediated (model binding),
//! so only unambiguous raw-request idioms participate, and everything ships
//! at `preview`.

use crate::review::signals::taint::sinks::{Sink, SinkKind};
use crate::review::signals::taint::tables::TaintTables;
use tree_sitter::Node;

pub(super) static CSHARP_TAINT: TaintTables = TaintTables {
    request_sources: &[
        "Request.Query",
        "Request.Form",
        "Request.Headers",
        "Request.Cookies",
        "Request.Body",
        "Request.QueryString",
        "HttpContext.Request",
    ],
    argv_sources: &["Environment.GetCommandLineArgs", "args"],
    // `Request.Query["id"]` reads through member access; the idiom text
    // lives on the member_access_expression node.
    source_access_kinds: &["member_access_expression"],
    // Coercion pruning is skipped: `int.Parse(...)` is an invocation the
    // shared walker cannot key off; a request value coerced to an int
    // rarely reaches a string-built sink.
    coercions: &[],
    coercion_call_kind: "invocation_expression",
    assignment_kinds: &["variable_declarator", "assignment_expression"],
    is_flow_scope,
    is_augmenting,
    is_string_building,
    classify_sink,
};

fn is_flow_scope(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "method_declaration"
            | "constructor_declaration"
            | "local_function_statement"
            | "lambda_expression"
            | "anonymous_method_expression"
    )
}

fn is_augmenting(node: Node<'_>) -> bool {
    node.kind() == "assignment_expression" && {
        let mut cursor = node.walk();
        node.children(&mut cursor).any(|child| {
            matches!(
                child.kind(),
                "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" | "??="
            )
        })
    }
}

fn is_string_building(node: Node<'_>, content: &str) -> bool {
    // `"..." + x` concatenation, `$"...{x}..."` interpolation, or
    // `string.Format(...)`.
    node.kind() == "binary_expression"
        || node.kind() == "interpolated_string_expression"
        || (node.kind() == "invocation_expression"
            && csharp_callee(node, content)
                .is_some_and(|callee| callee == "string.Format" || callee == "String.Format"))
}

/// The `Object.Method` callee text of an `invocation_expression`, with the
/// argument list stripped off (same approach as the Java frontend — the
/// shared `callee_text` helper only understands `call`/`call_expression`).
fn csharp_callee<'a>(node: Node<'a>, content: &'a str) -> Option<&'a str> {
    let args = node.child_by_field_name("arguments")?;
    content
        .get(node.start_byte()..args.start_byte())
        .map(str::trim)
        .filter(|callee| !callee.is_empty())
}

fn classify_sink<'a>(node: Node<'a>, content: &'a str) -> Option<Sink<'a>> {
    if node.kind() != "invocation_expression" {
        return None;
    }
    let args = node.child_by_field_name("arguments")?;
    let callee = csharp_callee(node, content)?;

    let kind = if callee.ends_with(".ExecuteReader")
        || callee.ends_with(".ExecuteNonQuery")
        || callee.ends_with(".ExecuteScalar")
        || callee.ends_with(".ExecuteReaderAsync")
        || callee.ends_with(".ExecuteNonQueryAsync")
        || callee.ends_with(".ExecuteScalarAsync")
    {
        SinkKind::Sql
    } else if callee == "Process.Start" {
        SinkKind::Exec
    } else if callee.starts_with("File.WriteAll")
        || callee.starts_with("File.AppendAll")
        || callee == "File.OpenWrite"
    {
        SinkKind::FsWrite
    } else {
        return None;
    };
    Some(Sink { kind, args })
}
