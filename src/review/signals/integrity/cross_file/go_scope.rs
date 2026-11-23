use crate::review::signals::content::ReviewSource;
use tree_sitter::Node;

pub(super) fn name_is_locally_bound(source: &ReviewSource, name: &str) -> bool {
    let Some(root) = source.tree().map(|tree| tree.root_node()) else {
        return false;
    };
    has_binding(root, source.content(), name)
}

fn has_binding(node: Node<'_>, content: &str, name: &str) -> bool {
    if binding_node(node).is_some_and(|binding| contains_identifier(binding, content, name)) {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| has_binding(child, content, name))
}

fn binding_node(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "short_var_declaration" | "range_clause" => node
            .child_by_field_name("left")
            .or_else(|| node.named_child(0)),
        "var_spec" | "const_spec" | "parameter_declaration" => node
            .child_by_field_name("name")
            .or_else(|| node.named_child(0)),
        _ => None,
    }
}

fn contains_identifier(node: Node<'_>, content: &str, expected: &str) -> bool {
    if node.kind() == "identifier"
        && node
            .utf8_text(content.as_bytes())
            .is_ok_and(|name| name == expected)
    {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| contains_identifier(child, content, expected))
}
