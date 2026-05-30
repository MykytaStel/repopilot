use crate::findings::types::Finding;
use std::path::Path;
use tree_sitter::{Node, Tree};

use super::LongFunctionPolicy;

/// Detects long functions from a parsed syntax tree.
///
/// Walks every function-like node (declarations, methods, named function
/// expressions, and arrow functions) and flags those whose line span exceeds
/// the policy threshold. Anonymous functions use a doubled threshold to match
/// the lower expectation for inline callbacks.
pub(super) fn detect_ast(
    tree: &Tree,
    content: &str,
    language: &str,
    path: &Path,
    policy: LongFunctionPolicy,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    visit(
        tree.root_node(),
        content,
        language,
        path,
        policy,
        &mut findings,
    );
    findings
}

fn visit(
    node: Node<'_>,
    content: &str,
    language: &str,
    path: &Path,
    policy: LongFunctionPolicy,
    findings: &mut Vec<Finding>,
) {
    if let Some((name, is_anonymous)) = function_like(node, language, content) {
        let start_row = node.start_position().row;
        let end_row = node.end_position().row;
        let fn_len = end_row.saturating_sub(start_row) + 1;

        // Inline callbacks are expected to be shorter; doubling the threshold
        // keeps them from dominating the signal, mirroring the prior heuristic.
        let threshold = if is_anonymous {
            policy.threshold.saturating_mul(2)
        } else {
            policy.threshold
        };

        if fn_len > threshold {
            let effective = LongFunctionPolicy {
                threshold,
                ..policy
            };
            findings.push(super::build_finding(
                path,
                start_row + 1,
                end_row + 1,
                &name,
                fn_len,
                effective,
            ));
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, content, language, path, policy, findings);
    }
}

/// Returns `(name, is_anonymous)` when `node` is a function-like construct for
/// `language`, or `None` otherwise. `name` is empty for anonymous functions.
fn function_like(node: Node<'_>, language: &str, content: &str) -> Option<(String, bool)> {
    match language {
        "Rust" => (node.kind() == "function_item")
            .then(|| (field_name(node, content).unwrap_or_default(), false)),
        "Python" => (node.kind() == "function_definition")
            .then(|| (field_name(node, content).unwrap_or_default(), false)),
        // TypeScript, TSX, JavaScript, and JSX share the same node kinds.
        _ => match node.kind() {
            "function_declaration" | "generator_function_declaration" | "method_definition" => {
                Some((field_name(node, content).unwrap_or_default(), false))
            }
            "function_expression" | "generator_function" => match field_name(node, content) {
                Some(name) => Some((name, false)),
                None => Some((String::new(), true)),
            },
            "arrow_function" => match arrow_function_name(node, content) {
                Some(name) => Some((name, false)),
                None => Some((String::new(), true)),
            },
            _ => None,
        },
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
