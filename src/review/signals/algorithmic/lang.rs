//! Node-kind matchers for algorithmic signals, driven by the language
//! frontend's [`AlgorithmicKinds`] table so the detector stays
//! language-neutral.

use crate::review::signals::tables::AlgorithmicKinds;
use tree_sitter::Node;

/// Returns the declared name when `node` is a named function per the table.
/// Anonymous functions / arrow callbacks return `None` — they cannot be matched
/// reliably across the change, and are not the focus of algorithmic signals.
pub(super) fn function_name(
    node: Node<'_>,
    kinds: &AlgorithmicKinds,
    content: &str,
) -> Option<String> {
    if !kinds.function_kinds.contains(&node.kind()) {
        return None;
    }
    let name = node.child_by_field_name("name")?;
    name.utf8_text(content.as_bytes())
        .ok()
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

pub(super) fn is_loop_node(kind: &str, kinds: &AlgorithmicKinds) -> bool {
    kinds.loop_kinds.contains(&kind)
}

pub(super) fn is_call_node(node: Node<'_>, kinds: &AlgorithmicKinds) -> bool {
    kinds.call_kinds.contains(&node.kind())
}

pub(super) fn is_control_flow_node(kind: &str, kinds: &AlgorithmicKinds) -> bool {
    kinds.control_flow_kinds.contains(&kind)
}

/// Whether `node` is the `if` of an `else if`, so an else-if chain counts as one
/// level of nesting rather than one per branch.
pub(super) fn is_else_if(node: Node<'_>, kinds: &AlgorithmicKinds) -> bool {
    if !kinds.if_kinds.contains(&node.kind()) {
        return false;
    }
    matches!(
        node.parent().map(|parent| parent.kind()),
        Some("else_clause" | "else")
    )
}
