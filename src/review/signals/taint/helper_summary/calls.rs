use super::super::super::tables::TaintTables;
use super::super::flow_assignments::{Assignment, collect_assignments};
use super::super::flow_seed::parameter_binding_name;
use super::Helper;
use tree_sitter::Node;

pub(super) fn collect_scopes<'a>(
    node: Node<'a>,
    tables: &'static TaintTables,
    out: &mut Vec<Node<'a>>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if (tables.is_flow_scope)(child) {
            out.push(child);
        }
        collect_scopes(child, tables, out);
    }
}

pub(super) fn collect_direct_calls<'a>(
    node: Node<'a>,
    scope: Node<'a>,
    content: &str,
    tables: &'static TaintTables,
    out: &mut Vec<Node<'a>>,
) {
    if node.id() != scope.id() && (tables.is_flow_scope)(node) {
        return;
    }
    if is_call(node) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if !inside_unsupported_control_flow(child, scope, tables) {
            collect_direct_calls(child, scope, content, tables, out);
        }
    }
    let _ = content;
}

fn inside_unsupported_control_flow(
    node: Node<'_>,
    scope: Node<'_>,
    tables: &'static TaintTables,
) -> bool {
    let mut current = Some(node);
    while let Some(candidate) = current {
        if candidate.id() == scope.id() {
            return false;
        }
        if matches!(
            candidate.kind(),
            "if_statement"
                | "if_expression"
                | "conditional_expression"
                | "ternary_expression"
                | "for_statement"
                | "for_in_statement"
                | "for_of_statement"
                | "while_statement"
                | "do_statement"
                | "loop_expression"
                | "switch_statement"
                | "match_statement"
                | "try_statement"
                | "select_statement"
                | "expression_switch_statement"
                | "type_switch_statement"
        ) {
            return true;
        }
        current = candidate.parent();
    }
    let _ = tables;
    false
}

pub(super) fn direct_callee<'a>(node: Node<'a>, content: &'a str) -> Option<&'a str> {
    if !is_call(node) {
        return None;
    }
    let callee = node.child_by_field_name("function")?;
    (callee.kind() == "identifier").then(|| callee.utf8_text(content.as_bytes()).ok())?
}

fn is_call(node: Node<'_>) -> bool {
    matches!(node.kind(), "call" | "call_expression")
}

pub(super) fn caller_shadowed(
    caller: Node<'_>,
    helper: &Helper<'_>,
    content: &str,
    tables: &'static TaintTables,
) -> bool {
    if caller
        .child_by_field_name("parameters")
        .is_some_and(|parameters| {
            let mut cursor = parameters.walk();
            parameters.named_children(&mut cursor).any(|parameter| {
                parameter_binding_name(parameter, content).as_deref() == Some(helper.name.as_str())
            })
        })
    {
        return true;
    }
    let mut assignments: Vec<Assignment<'_>> = Vec::new();
    collect_assignments(caller, tables, content, &mut assignments);
    assignments
        .iter()
        .any(|assignment| assignment.names.iter().any(|name| name == &helper.name))
}

pub(super) fn bound_arguments<'tree>(
    call: Node<'tree>,
    parameters: &[String],
    content: &str,
) -> Option<Vec<Node<'tree>>> {
    let arguments = call.child_by_field_name("arguments")?;
    let mut cursor = arguments.walk();
    let mut bound = vec![None; parameters.len()];
    let mut next_position = 0;
    let mut saw_keyword = false;
    for argument in arguments
        .named_children(&mut cursor)
        .filter(|node| !node.is_extra())
    {
        let (position, value) = match argument.kind() {
            "keyword_argument" => {
                saw_keyword = true;
                let name = argument
                    .child_by_field_name("name")?
                    .utf8_text(content.as_bytes())
                    .ok()?;
                let position = parameters.iter().position(|parameter| parameter == name)?;
                (position, argument.child_by_field_name("value")?)
            }
            "list_splat" | "dictionary_splat" | "spread_element" | "variadic_argument" => {
                return None;
            }
            _ if saw_keyword => return None,
            _ => {
                let position = next_position;
                next_position += 1;
                (position, argument)
            }
        };
        if bound.get_mut(position)?.replace(value).is_some() {
            return None;
        }
    }
    bound.into_iter().collect()
}

#[path = "calls/apply.rs"]
mod apply;
pub(super) use apply::apply_helper_call;
