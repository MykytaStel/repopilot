use super::super::sources::node_has_source;
use super::super::tables::TaintTables;
use super::flow_state::{TaintState, access_path, value_path};
use tree_sitter::Node;

pub(super) struct DestructuredBinding {
    pub(super) name: String,
    pub(super) path: String,
}

pub(super) struct Assignment<'a> {
    pub(super) names: Vec<String>,
    pub(super) bindings: Vec<DestructuredBinding>,
    pub(super) path: Option<String>,
    pub(super) rhs: Node<'a>,
    pub(super) augmenting: bool,
}

pub(super) fn collect_assignments<'a>(
    node: Node<'a>,
    tables: &'static TaintTables,
    content: &str,
    out: &mut Vec<Assignment<'a>>,
) {
    if let Some((lhs, rhs)) = assignment_parts(node, tables) {
        let mut names = Vec::new();
        let mut bindings = Vec::new();
        match lhs.kind() {
            "object_pattern" => collect_object_bindings(lhs, content, "", &mut bindings),
            "array_pattern" => collect_array_bindings(lhs, content, "", &mut bindings),
            _ => collect_lhs_names(lhs, content, &mut names),
        }
        let path = access_path(lhs, content);
        if !names.is_empty() || !bindings.is_empty() || path.is_some() {
            out.push(Assignment {
                names,
                bindings,
                path,
                rhs,
                augmenting: (tables.is_augmenting)(node),
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !(tables.is_flow_scope)(child) {
            collect_assignments(child, tables, content, out);
        }
    }
}

fn assignment_parts<'a>(
    node: Node<'a>,
    tables: &'static TaintTables,
) -> Option<(Node<'a>, Node<'a>)> {
    let kind = node.kind();
    if !tables.assignment_kinds.contains(&kind) {
        return None;
    }
    let (lhs_field, rhs_field) = if kind == "variable_declarator" {
        ("name", "value")
    } else {
        ("left", "right")
    };
    let lhs = node.child_by_field_name(lhs_field)?;
    if let Some(rhs) = node.child_by_field_name(rhs_field) {
        return Some((lhs, rhs));
    }
    if kind == "variable_declarator" {
        let mut cursor = node.walk();
        let init = node
            .named_children(&mut cursor)
            .filter(|child| child.id() != lhs.id())
            .last()?;
        return Some((lhs, init));
    }
    None
}

fn collect_object_bindings(
    node: Node<'_>,
    content: &str,
    prefix: &str,
    out: &mut Vec<DestructuredBinding>,
) {
    match node.kind() {
        "shorthand_property_identifier_pattern" => {
            if let Ok(name) = node.utf8_text(content.as_bytes()) {
                let path = join_binding_path(prefix, name);
                out.push(DestructuredBinding {
                    name: name.to_string(),
                    path,
                });
            }
        }
        "pair_pattern" => {
            let Some(key) = node.child_by_field_name("key") else {
                return;
            };
            let Some(value) = node.child_by_field_name("value") else {
                return;
            };
            let Ok(key_text) = key.utf8_text(content.as_bytes()) else {
                return;
            };
            if !is_identifier(key_text) {
                return;
            }
            let path = join_binding_path(prefix, key_text);
            collect_binding_value(value, content, &path, out);
        }
        "object_pattern" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_object_bindings(child, content, prefix, out);
            }
        }
        _ => {}
    }
}

fn collect_array_bindings(
    node: Node<'_>,
    content: &str,
    prefix: &str,
    out: &mut Vec<DestructuredBinding>,
) {
    let mut cursor = node.walk();
    for (index, child) in node.named_children(&mut cursor).enumerate() {
        if child.kind() == "rest_pattern" {
            continue;
        }
        let path = join_binding_path(prefix, &index.to_string());
        collect_binding_value(child, content, &path, out);
    }
}

fn collect_binding_value(
    node: Node<'_>,
    content: &str,
    path: &str,
    out: &mut Vec<DestructuredBinding>,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            if let Ok(name) = node.utf8_text(content.as_bytes()) {
                out.push(DestructuredBinding {
                    name: name.to_string(),
                    path: path.to_string(),
                });
            }
        }
        "object_pattern" => collect_object_bindings(node, content, path, out),
        "array_pattern" => collect_array_bindings(node, content, path, out),
        "assignment_pattern" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_binding_value(left, content, path, out);
            }
        }
        _ => {}
    }
}

fn join_binding_path(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_string()
    } else {
        format!("{prefix}.{segment}")
    }
}

fn collect_lhs_names(lhs: Node<'_>, content: &str, out: &mut Vec<String>) {
    match lhs.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            if let Ok(text) = lhs.utf8_text(content.as_bytes()) {
                out.push(text.to_string());
            }
        }
        "object_pattern"
        | "array_pattern"
        | "object_assignment_pattern"
        | "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "expression_list" => {
            let mut cursor = lhs.walk();
            for child in lhs.named_children(&mut cursor) {
                collect_lhs_names(child, content, out);
            }
        }
        "pair_pattern" => {
            if let Some(value) = lhs.child_by_field_name("value") {
                collect_lhs_names(value, content, out);
            }
        }
        _ => {}
    }
}

pub(super) fn apply_assignment(
    assignment: &Assignment<'_>,
    content: &str,
    tables: &'static TaintTables,
    tainted: &mut TaintState,
) {
    let source = node_has_source(assignment.rhs, content, tables).or_else(|| {
        super::flow_checks::node_mentions_tainted(assignment.rhs, content, tables, tainted)
    });

    for name in &assignment.names {
        match source {
            Some(kind) => tainted.mark_name(name, kind),
            None if !assignment.augmenting => tainted.clear_name(name),
            None => {}
        }
    }

    let rhs_path = value_path(assignment.rhs, content);
    for binding in &assignment.bindings {
        let binding_source = match rhs_path.as_deref() {
            Some(base) => {
                let path = format!("{base}.{}", binding.path);
                if tainted.is_path_explicitly_clean(&path) {
                    None
                } else {
                    tainted.source_for_path(&path).or(source)
                }
            }
            None => source,
        };
        match binding_source {
            Some(kind) => tainted.mark_name(&binding.name, kind),
            None if !assignment.augmenting => tainted.clear_name(&binding.name),
            None => {}
        }
    }

    if let Some(path) = &assignment.path {
        match source {
            Some(kind) => tainted.mark_path(path, kind),
            None if !assignment.augmenting => tainted.clear_path(path),
            None => {}
        }
    }
}

fn is_identifier(segment: &str) -> bool {
    let mut chars = segment.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    (first.is_ascii_alphabetic() || matches!(first, '_' | '$'))
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$'))
}
