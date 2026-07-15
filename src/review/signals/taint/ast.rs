use tree_sitter::Node;

pub(crate) fn first_named_arg(args: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = args.walk();
    args.named_children(&mut cursor).next()
}

pub(crate) fn has_descendant_kind(node: Node<'_>, kind: &str) -> bool {
    if node.kind() == kind {
        return true;
    }
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| has_descendant_kind(child, kind))
}

fn callee_text<'a>(node: Node<'a>, content: &'a str) -> Option<&'a str> {
    node.child_by_field_name("function")?
        .utf8_text(content.as_bytes())
        .ok()
        .map(str::trim)
}

pub(crate) fn callee_ends_with(node: Node<'_>, content: &str, suffix: &str) -> bool {
    callee_text(node, content).is_some_and(|text| text.ends_with(suffix))
}

pub(crate) fn callee_starts_with(node: Node<'_>, content: &str, prefix: &str) -> bool {
    callee_text(node, content).is_some_and(|text| text.starts_with(prefix))
}
