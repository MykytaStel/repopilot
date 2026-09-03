use super::super::sources::SourceKind;
use super::super::tables::TaintTables;
use super::flow_assignments::{Assignment, collect_assignments};
use super::flow_state::TaintState;
use tree_sitter::Node;

pub(super) fn seed_tainted(
    root: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
) -> TaintState {
    let mut tainted = TaintState::default();
    collect_request_bound_parameters(root, content, tables, &mut tainted);

    let mut assignments: Vec<Assignment<'_>> = Vec::new();
    collect_assignments(root, tables, content, &mut assignments);
    assignments.sort_by_key(|assignment| assignment.rhs.start_byte());

    for assignment in &assignments {
        super::flow_assignments::apply_assignment(assignment, content, tables, &mut tainted);
    }
    tainted
}

fn collect_request_bound_parameters(
    node: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    out: &mut TaintState,
) {
    if is_parameter_node(node) {
        let text = node.utf8_text(content.as_bytes()).unwrap_or("");
        if is_nest_request_parameter(text)
            && !has_nest_primitive_pipe(text)
            && let Some(name) = parameter_binding_name(node, content)
        {
            out.mark_name(&name, SourceKind::HttpRequest);
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if !(tables.is_flow_scope)(child) {
            collect_request_bound_parameters(child, content, tables, out);
        }
    }
}

fn is_parameter_node(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "required_parameter" | "optional_parameter" | "rest_pattern"
    )
}

fn is_nest_request_parameter(text: &str) -> bool {
    ["Body", "Query", "Param", "Headers", "Req", "Request"]
        .iter()
        .any(|name| contains_decorator_token(text, name))
}

pub(super) fn has_nest_primitive_pipe(text: &str) -> bool {
    [
        "ParseIntPipe",
        "ParseFloatPipe",
        "ParseBoolPipe",
        "ParseUUIDPipe",
    ]
    .iter()
    .any(|pipe| contains_identifier_token(text, pipe))
}

fn contains_decorator_token(text: &str, name: &str) -> bool {
    let marker = format!("@{name}");
    text.match_indices(&marker).any(|(start, _)| {
        let suffix = &text[start + marker.len()..];
        suffix
            .chars()
            .next()
            .is_none_or(|next| next == '(' || next.is_whitespace())
    })
}

fn contains_identifier_token(text: &str, identifier: &str) -> bool {
    text.match_indices(identifier).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + identifier.len()..].chars().next();
        before.is_none_or(|ch| !is_identifier_char(ch))
            && after.is_none_or(|ch| !is_identifier_char(ch))
    })
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$')
}

fn parameter_binding_name(node: Node<'_>, content: &str) -> Option<String> {
    for field in ["pattern", "name"] {
        if let Some(binding) = node.child_by_field_name(field)
            && let Some(name) = first_binding_identifier(binding, content)
        {
            return Some(name);
        }
    }

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| !matches!(child.kind(), "decorator" | "type_annotation"))
        .find_map(|child| first_binding_identifier(child, content))
}

fn first_binding_identifier(node: Node<'_>, content: &str) -> Option<String> {
    if matches!(
        node.kind(),
        "identifier" | "shorthand_property_identifier_pattern"
    ) {
        return node
            .utf8_text(content.as_bytes())
            .ok()
            .map(ToOwned::to_owned);
    }
    if matches!(node.kind(), "decorator" | "type_annotation") {
        return None;
    }

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find_map(|child| first_binding_identifier(child, content))
}
