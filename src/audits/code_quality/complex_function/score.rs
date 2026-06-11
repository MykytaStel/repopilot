//! Cognitive-complexity-lite scoring for a single function body.
//!
//! Each control-flow structure costs `1 + its nesting depth`, so a deeply
//! nested handler scores high while a wide-but-flat dispatcher (a `switch`/
//! `match` with many shallow arms) scores ~1 — the property that sets this rule
//! apart from `code-quality.complex-file`'s flat branch count. A boolean
//! operator (`&&`/`||`, `and`/`or`) costs 1. Nested functions are *not* folded
//! in; they are scored independently by the caller's `for_each_function` walk.
//!
//! The control-flow node-kind tables mirror `deep_control_flow.rs` (the two
//! rules ask different questions of the same structures and are kept separate).

use crate::audits::code_quality::function_spans::is_function_node;
use tree_sitter::Node;

/// Cognitive-complexity-lite score for the body of `function_node`.
pub(super) fn cognitive_score(function_node: Node<'_>, language: &str) -> usize {
    let mut score = 0;
    // The function node itself is not a nesting structure; start at its children.
    let mut cursor = function_node.walk();
    for child in function_node.children(&mut cursor) {
        visit(child, 0, language, &mut score);
    }
    score
}

fn visit(node: Node<'_>, depth: usize, language: &str, score: &mut usize) {
    // A nested function is its own unit; the caller scores it separately.
    if is_function_node(node, language) {
        return;
    }

    let mut child_depth = depth;
    if is_control_flow_node(node.kind(), language) {
        if is_else_if(node, language) {
            // A branch, but at the same nesting level as its `if`.
            *score += 1;
        } else {
            *score += 1 + depth;
            child_depth = depth + 1;
        }
    } else if is_logical_operator(node, language) {
        *score += 1;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, child_depth, language, score);
    }
}

fn is_logical_operator(node: Node<'_>, language: &str) -> bool {
    match language {
        "Python" => node.kind() == "boolean_operator",
        _ => {
            node.kind() == "binary_expression" && {
                let mut cursor = node.walk();
                node.children(&mut cursor)
                    .any(|child| matches!(child.kind(), "&&" | "||"))
            }
        }
    }
}

fn is_control_flow_node(kind: &str, language: &str) -> bool {
    match language {
        "Rust" => matches!(
            kind,
            "if_expression"
                | "for_expression"
                | "while_expression"
                | "loop_expression"
                | "match_expression"
        ),
        "Python" => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "while_statement"
                | "match_statement"
                | "try_statement"
        ),
        "Go" => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "expression_switch_statement"
                | "type_switch_statement"
                | "select_statement"
        ),
        "Java" => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "enhanced_for_statement"
                | "while_statement"
                | "do_statement"
                | "switch_statement"
                | "try_statement"
        ),
        "CSharp" | "C#" => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "foreach_statement"
                | "while_statement"
                | "do_statement"
                | "switch_statement"
                | "try_statement"
        ),
        "Kotlin" => matches!(
            kind,
            "if_expression"
                | "when_expression"
                | "for_statement"
                | "while_statement"
                | "do_while_statement"
                | "try_expression"
        ),
        _ => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "for_in_statement"
                | "for_of_statement"
                | "while_statement"
                | "do_statement"
                | "switch_statement"
                | "try_statement"
        ),
    }
}

fn is_else_if(node: Node<'_>, language: &str) -> bool {
    let kind = node.kind();
    let is_if = match language {
        "Rust" | "Kotlin" => kind == "if_expression",
        _ => kind == "if_statement",
    };
    if is_if && let Some(parent) = node.parent() {
        if parent.kind() == "else_clause" || parent.kind() == "else" {
            return true;
        }
        if language == "Kotlin" && parent.kind() == "if_expression" {
            let mut cursor = parent.walk();
            let mut saw_else = false;
            for child in parent.children(&mut cursor) {
                if child.kind() == "else" {
                    saw_else = true;
                } else if child.id() == node.id() {
                    return saw_else;
                }
            }
        }
    }
    false
}
