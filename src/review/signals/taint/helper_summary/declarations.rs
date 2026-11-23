use super::super::super::tables::TaintTables;
use super::super::flow_assignments::{Assignment, collect_assignments};
use super::super::flow_seed::parameter_binding_name;
use super::{Helper, MAX_HELPERS, ReturnFlow};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub(super) fn unique_helpers<'a>(
    root: Node<'a>,
    content: &str,
    tables: &'static TaintTables,
) -> Vec<Helper<'a>> {
    let mut declarations = Vec::new();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        collect_exported_definition(child, &mut declarations);
        if declarations.len() > MAX_HELPERS {
            return Vec::new();
        }
    }
    let mut counts = BTreeMap::<String, usize>::new();
    let mut helpers = Vec::new();
    for node in declarations {
        if !(tables.is_flow_scope)(node)
            || !matches!(
                node.kind(),
                "function_declaration" | "generator_function_declaration" | "function_definition"
            )
        {
            continue;
        }
        let Some(name) = node
            .child_by_field_name("name")
            .and_then(|name| name.utf8_text(content.as_bytes()).ok())
            .map(str::to_string)
        else {
            continue;
        };
        *counts.entry(name.clone()).or_default() += 1;
        let Some(parameters) = helper_parameters(node, content) else {
            continue;
        };
        helpers.push(Helper {
            name,
            node,
            return_flow: returned_flow(node, &parameters, content, tables),
            parameters,
        });
    }
    helpers.retain(|helper| counts.get(&helper.name) == Some(&1));
    helpers
}

fn collect_exported_definition<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if matches!(
        node.kind(),
        "function_declaration" | "generator_function_declaration" | "function_definition"
    ) {
        out.push(node);
        return;
    }
    if matches!(node.kind(), "export_statement" | "decorated_definition") {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            collect_exported_definition(child, out);
        }
    }
}

fn helper_parameters(node: Node<'_>, content: &str) -> Option<Vec<String>> {
    let parameters = node.child_by_field_name("parameters")?;
    let mut cursor = parameters.walk();
    parameters
        .named_children(&mut cursor)
        .try_fold(Vec::new(), |mut out, parameter| {
            if matches!(
                parameter.kind(),
                "rest_pattern"
                    | "variadic_parameter"
                    | "list_splat_pattern"
                    | "dictionary_splat_pattern"
            ) {
                return None;
            }
            if parameter.kind() == "parameter_declaration" {
                let mut cursor = parameter.walk();
                out.extend(
                    parameter
                        .named_children(&mut cursor)
                        .filter(|child| child.kind() == "identifier")
                        .map(|name| name.utf8_text(content.as_bytes()).ok().map(str::to_string))
                        .collect::<Option<Vec<_>>>()?,
                );
                return Some(out);
            }
            let name = if parameter.kind() == "identifier" {
                parameter
                    .utf8_text(content.as_bytes())
                    .ok()
                    .map(str::to_string)
            } else {
                parameter_binding_name(parameter, content)
            }?;
            out.push(name);
            Some(out)
        })
}

fn returned_flow(
    helper: Node<'_>,
    parameters: &[String],
    content: &str,
    tables: &'static TaintTables,
) -> ReturnFlow {
    let Some(body) = helper.child_by_field_name("body") else {
        return ReturnFlow::Unknown;
    };
    let mut returns = Vec::new();
    collect_returns(body, tables, &mut returns);
    match returns.as_slice() {
        [] => ReturnFlow::Clean,
        [returned] => returned_value_flow(*returned, helper, parameters, content, tables),
        _ => ReturnFlow::Unknown,
    }
}

fn returned_value_flow(
    returned: Node<'_>,
    helper: Node<'_>,
    parameters: &[String],
    content: &str,
    tables: &'static TaintTables,
) -> ReturnFlow {
    let mut cursor = returned.walk();
    let mut values = returned.named_children(&mut cursor);
    let Some(value) = values.next() else {
        return ReturnFlow::Clean;
    };
    if values.next().is_some() {
        return ReturnFlow::Unknown;
    }
    if is_clean_literal(value) {
        return ReturnFlow::Clean;
    }
    returned_parameter_flow(value, helper, parameters, content, tables)
}

fn returned_parameter_flow(
    value: Node<'_>,
    helper: Node<'_>,
    parameters: &[String],
    content: &str,
    tables: &'static TaintTables,
) -> ReturnFlow {
    if value.kind() != "identifier" {
        return ReturnFlow::Unknown;
    }
    let Some(name) = value.utf8_text(content.as_bytes()).ok() else {
        return ReturnFlow::Unknown;
    };
    let Some(parameter_index) = parameters.iter().position(|parameter| parameter == name) else {
        return ReturnFlow::Unknown;
    };
    let mut assignments: Vec<Assignment<'_>> = Vec::new();
    collect_assignments(helper, tables, content, &mut assignments);
    let writes = assignments
        .iter()
        .filter(|assignment| assignment.names.iter().any(|assigned| assigned == name))
        .collect::<Vec<_>>();
    if writes.is_empty() {
        return ReturnFlow::Parameter(parameter_index);
    }
    if writes
        .iter()
        .all(|assignment| is_clean_literal(assignment.rhs))
    {
        ReturnFlow::Clean
    } else {
        ReturnFlow::Unknown
    }
}

fn is_clean_literal(node: Node<'_>) -> bool {
    if node.kind() == "string" {
        let mut cursor = node.walk();
        return !node
            .named_children(&mut cursor)
            .any(|child| child.kind() == "interpolation");
    }
    matches!(
        node.kind(),
        "string_literal"
            | "interpreted_string_literal"
            | "raw_string_literal"
            | "number"
            | "integer"
            | "float"
            | "true"
            | "false"
            | "null"
            | "none"
            | "nil"
    )
}

fn collect_returns<'a>(node: Node<'a>, tables: &'static TaintTables, out: &mut Vec<Node<'a>>) {
    if node.kind() == "return_statement" {
        out.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if !(tables.is_flow_scope)(child) {
            collect_returns(child, tables, out);
        }
    }
}

pub(super) fn unsupported_control_flow(node: Node<'_>, tables: &'static TaintTables) -> bool {
    const CONTROL_FLOW: &[&str] = &[
        "if_statement",
        "if_expression",
        "conditional_expression",
        "ternary_expression",
        "for_statement",
        "for_in_statement",
        "for_of_statement",
        "while_statement",
        "do_statement",
        "loop_expression",
        "switch_statement",
        "match_statement",
        "try_statement",
        "select_statement",
        "expression_switch_statement",
        "type_switch_statement",
    ];
    if CONTROL_FLOW.contains(&node.kind()) {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| (tables.is_flow_scope)(child) || unsupported_control_flow(child, tables))
}

pub(super) fn calls_local_helper(
    node: Node<'_>,
    content: &str,
    tables: &'static TaintTables,
    names: &BTreeSet<String>,
) -> bool {
    if super::calls::direct_callee(node, content).is_some_and(|name| names.contains(name)) {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor).any(|child| {
        !(tables.is_flow_scope)(child) && calls_local_helper(child, content, tables, names)
    })
}
