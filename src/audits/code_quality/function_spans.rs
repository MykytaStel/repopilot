//! Shared function-boundary walker for AST-based code-quality audits.
//!
//! `long-function` and `complex-function` must agree on what counts as a
//! "function" across the supported languages, so the node-kind knowledge lives
//! here once. `for_each_function` visits every function-like node (descending
//! into nested functions); `is_function_node` lets a caller treat a nested
//! function as a boundary without re-deriving the kind list.

use tree_sitter::{Node, Tree};

/// Invoke `callback(node, name, is_anonymous)` for every function-like node in
/// `tree`. `name` is empty for true anonymous callbacks. Nested functions are
/// visited too, so each function is reported in its own right.
pub(super) fn for_each_function(
    tree: &Tree,
    content: &str,
    language: &str,
    callback: &mut dyn FnMut(Node<'_>, &str, bool),
) {
    visit(tree.root_node(), content, language, callback);
}

fn visit(
    node: Node<'_>,
    content: &str,
    language: &str,
    callback: &mut dyn FnMut(Node<'_>, &str, bool),
) {
    if let Some((name, is_anonymous)) = function_like(node, language, content) {
        callback(node, &name, is_anonymous);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, content, language, callback);
    }
}

/// Whether `node` is a function-like construct for `language`. Used by callers
/// that need to stop at a nested-function boundary.
pub(super) fn is_function_node(node: Node<'_>, language: &str) -> bool {
    match language {
        "Rust" => node.kind() == "function_item",
        "Python" => node.kind() == "function_definition",
        "Go" => matches!(node.kind(), "function_declaration" | "method_declaration"),
        "Java" => matches!(
            node.kind(),
            "method_declaration" | "constructor_declaration"
        ),
        "CSharp" | "C#" => matches!(
            node.kind(),
            "method_declaration" | "constructor_declaration" | "local_function_statement"
        ),
        "Kotlin" => node.kind() == "function_declaration",
        // TypeScript, TSX, JavaScript, and JSX share the same node kinds.
        _ => matches!(
            node.kind(),
            "function_declaration"
                | "generator_function_declaration"
                | "method_definition"
                | "function_expression"
                | "generator_function"
                | "arrow_function"
        ),
    }
}

/// Returns `(name, is_anonymous)` when `node` is a function-like construct for
/// `language`, or `None` otherwise. `name` is empty for anonymous functions.
fn function_like(node: Node<'_>, language: &str, content: &str) -> Option<(String, bool)> {
    if !is_function_node(node, language) {
        return None;
    }

    // Anonymity only applies to the JS/TS function expression and arrow forms;
    // every other function-like node is named (or empty-named).
    match node.kind() {
        "function_expression" | "generator_function" => match field_name(node, content) {
            Some(name) => Some((name, false)),
            None => Some((String::new(), true)),
        },
        "arrow_function" => match arrow_function_name(node, content) {
            Some(name) => Some((name, false)),
            None => Some((String::new(), true)),
        },
        _ => Some((field_name(node, content).unwrap_or_default(), false)),
    }
}

fn field_name(node: Node<'_>, content: &str) -> Option<String> {
    let name = node.child_by_field_name("name")?;
    name.utf8_text(content.as_bytes()).ok().map(str::to_string)
}

/// Derives a name for an arrow function from its binding context (e.g.
/// `const handler = () => {}` or an object property), returning `None` for
/// true inline callbacks.
fn arrow_function_name(node: Node<'_>, content: &str) -> Option<String> {
    let parent = node.parent()?;
    let field = match parent.kind() {
        "variable_declarator" | "public_field_definition" | "field_definition" => "name",
        "pair" => "key",
        "assignment_expression" => "left",
        _ => return None,
    };
    let name = parent.child_by_field_name(field)?;
    name.utf8_text(content.as_bytes()).ok().map(str::to_string)
}
