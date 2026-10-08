//! Review-only candidates for changed empty catch handlers followed by a return.

mod python;

use crate::audits::context::classify::helpers::is_test_file;
use crate::review::diff::ChangedFile;
use crate::review::signals::behavioral::{
    BehavioralKind, BehavioralSignal, BehavioralSignalSource,
};
use crate::review::signals::content::ReviewSource;
use std::path::Path;
use tree_sitter::Node;

const JS_TS_EXTENSIONS: &[&str] = &["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"];

/// Find changed empty catch handlers that can continue to a later return.
pub(crate) fn detect_quiet_fallback(
    file: &ChangedFile,
    post: &ReviewSource,
) -> Vec<BehavioralSignal> {
    if is_test_file(&file.path) {
        return Vec::new();
    }
    if is_python_file(&file.path, post.language_label()) {
        return python::detect(file, post);
    }
    if !is_js_ts_file(&file.path, post.language_label()) {
        return Vec::new();
    }
    let Some(tree) = post.tree().filter(|tree| !tree.root_node().has_error()) else {
        return Vec::new();
    };

    let mut signals = Vec::new();
    visit(tree.root_node(), file, post.content(), &mut signals);
    signals
}

fn is_python_file(path: &Path, label: Option<&str>) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("py"))
        && label.is_some_and(|value| value.eq_ignore_ascii_case("python"))
}

fn is_js_ts_file(path: &Path, label: Option<&str>) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    let extension = extension.to_ascii_lowercase();
    let language = label.unwrap_or_default().to_ascii_lowercase();
    JS_TS_EXTENSIONS.contains(&extension.as_str())
        && matches!(
            language.as_str(),
            "javascript" | "typescript" | "javascript-react" | "typescript-react"
        )
}

fn visit(node: Node<'_>, file: &ChangedFile, content: &str, signals: &mut Vec<BehavioralSignal>) {
    if node.kind() == "try_statement"
        && let Some(signal) = quiet_fallback(node, file, content)
    {
        signals.push(signal);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, file, content, signals);
    }
}

fn quiet_fallback(
    try_node: Node<'_>,
    file: &ChangedFile,
    content: &str,
) -> Option<BehavioralSignal> {
    let catch = named_child(try_node, "catch_clause")?;
    let line = catch.start_position().row + 1;
    let call = first_call(try_body(try_node)?, true)?;
    if !file.contains_line(line) || !empty_catch(catch) {
        return None;
    }
    let function = enclosing_function(try_node)?;
    let return_line = later_return(function, try_node.end_byte(), true)?;
    let call_line = call.start_position().row + 1;
    let operation = call
        .child_by_field_name("function")
        .and_then(|node| node.utf8_text(content.as_bytes()).ok())
        .map(compact_operation)
        .unwrap_or_else(|| "operation".to_string());

    Some(BehavioralSignal {
        kind: BehavioralKind::QuietFallbackIntroduced,
        path: file.path_string(),
        line,
        detail: format!(
            "Call `{operation}` at line {call_line} can fail inside the changed empty catch; execution may continue to return line {return_line}. Verify the fallback preserves required behavior."
        ),
        source: BehavioralSignalSource::Ast,
    })
}

fn named_child<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == kind)
}

fn try_body<'tree>(node: Node<'tree>) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == "statement_block")
}

fn empty_catch(catch: Node<'_>) -> bool {
    let Some(body) = try_body(catch) else {
        return false;
    };
    let mut cursor = body.walk();
    body.named_children(&mut cursor)
        .all(|child| child.kind() == "comment")
}

fn first_call(node: Node<'_>, is_root: bool) -> Option<Node<'_>> {
    if node.kind() == "call_expression" {
        return Some(node);
    }
    if !is_root && is_function_scope(node.kind()) {
        return None;
    }
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find_map(|child| first_call(child, false))
}

fn compact_operation(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(80).collect()
}

fn enclosing_function(node: Node<'_>) -> Option<Node<'_>> {
    let mut parent = node.parent();
    while let Some(candidate) = parent {
        if is_function_scope(candidate.kind()) {
            return Some(candidate);
        }
        parent = candidate.parent();
    }
    None
}

fn later_return(node: Node<'_>, after_byte: usize, is_root: bool) -> Option<usize> {
    if node.kind() == "return_statement" && node.start_byte() > after_byte {
        return Some(node.start_position().row + 1);
    }
    if !is_root && is_function_scope(node.kind()) {
        return None;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(line) = later_return(child, after_byte, false) {
            return Some(line);
        }
    }
    None
}

fn is_function_scope(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration"
            | "generator_function_declaration"
            | "method_definition"
            | "function_expression"
            | "generator_function"
            | "arrow_function"
    )
}
