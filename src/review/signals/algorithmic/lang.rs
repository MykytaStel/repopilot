//! Per-language tree-sitter node-kind matchers for algorithmic signals.
//!
//! Kept separate from the detection logic so the detector stays focused and each
//! file stays small. These mirror the node sets the code-quality audits use for
//! the same nine grammars; duplicated here to keep the review signal
//! self-contained rather than reaching into the audit internals.

use tree_sitter::Node;

/// Returns the declared name when `node` is a named function for `language`.
/// Anonymous functions / arrow callbacks return `None` — they cannot be matched
/// reliably across the change, and are not the focus of algorithmic signals.
pub(super) fn function_name(node: Node<'_>, language: &str, content: &str) -> Option<String> {
    let is_fn = match language {
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
        _ => matches!(
            node.kind(),
            "function_declaration" | "generator_function_declaration" | "method_definition"
        ),
    };
    if !is_fn {
        return None;
    }
    let name = node.child_by_field_name("name")?;
    name.utf8_text(content.as_bytes())
        .ok()
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

pub(super) fn is_loop_node(kind: &str, language: &str) -> bool {
    match language {
        "Rust" => matches!(
            kind,
            "for_expression" | "while_expression" | "loop_expression"
        ),
        "Python" => matches!(kind, "for_statement" | "while_statement"),
        "Go" => kind == "for_statement",
        "Java" => matches!(
            kind,
            "for_statement" | "enhanced_for_statement" | "while_statement" | "do_statement"
        ),
        "CSharp" | "C#" => matches!(
            kind,
            "for_statement" | "foreach_statement" | "while_statement" | "do_statement"
        ),
        "Kotlin" => matches!(
            kind,
            "for_statement" | "while_statement" | "do_while_statement"
        ),
        _ => matches!(
            kind,
            "for_statement"
                | "for_in_statement"
                | "for_of_statement"
                | "while_statement"
                | "do_statement"
        ),
    }
}

pub(super) fn is_control_flow_node(kind: &str, language: &str) -> bool {
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

/// Whether `node` is the `if` of an `else if`, so an else-if chain counts as one
/// level of nesting rather than one per branch.
pub(super) fn is_else_if(node: Node<'_>, language: &str) -> bool {
    let is_if = match language {
        "Rust" | "Kotlin" => node.kind() == "if_expression",
        _ => node.kind() == "if_statement",
    };
    if !is_if {
        return false;
    }
    matches!(
        node.parent().map(|parent| parent.kind()),
        Some("else_clause" | "else")
    )
}
