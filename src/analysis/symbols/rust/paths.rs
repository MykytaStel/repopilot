use super::super::{JavaScriptSymbolFacts, RustCallFact, RustFieldUseFact};
use super::{Node, text};
use std::collections::BTreeSet;

pub(super) fn record_call(
    path: Node<'_>,
    arguments: Node<'_>,
    content: &str,
    modules: &BTreeSet<&str>,
    facts: &mut JavaScriptSymbolFacts,
) {
    let Some((imported_name, module_specifier)) = resolved_path(path, content, modules) else {
        return;
    };
    let mut cursor = arguments.walk();
    let argument_count = arguments.named_children(&mut cursor).count();
    let Some(contracts) = facts.rust_contracts.as_mut() else {
        return;
    };
    contracts.calls.push(RustCallFact {
        imported_name: imported_name.into(),
        module_specifier,
        line_start: path.start_position().row + 1,
        line_end: path.end_position().row + 1,
        byte_start: path.start_byte(),
        byte_end: path.end_byte(),
        argument_count,
    });
}

pub(super) fn record_struct_field_uses(
    node: Node<'_>,
    content: &str,
    modules: &BTreeSet<&str>,
    facts: &mut JavaScriptSymbolFacts,
) {
    if node.kind() == "struct_expression"
        && let Some(name) = node.child_by_field_name("name")
        && let Some(body) = node.child_by_field_name("body")
    {
        let Some((struct_name, module_specifier)) = resolved_path(name, content, modules) else {
            return;
        };
        let mut cursor = body.walk();
        for field in body.named_children(&mut cursor) {
            let field_name = match field.kind() {
                "field_initializer" => field.child_by_field_name("field"),
                "shorthand_field_initializer" => field.named_child(0),
                _ => None,
            };
            let Some(field_name) = field_name else {
                continue;
            };
            let Some(contracts) = facts.rust_contracts.as_mut() else {
                return;
            };
            contracts.struct_field_uses.push(RustFieldUseFact {
                struct_name: struct_name.into(),
                field_name: text(field_name, content).into(),
                module_specifier: module_specifier.clone(),
                line_start: field_name.start_position().row + 1,
                line_end: field_name.end_position().row + 1,
                byte_start: field_name.start_byte(),
                byte_end: field_name.end_byte(),
            });
        }
    }
}

pub(super) fn resolved_path<'a>(
    path: Node<'_>,
    content: &'a str,
    modules: &BTreeSet<&str>,
) -> Option<(&'a str, String)> {
    let segments = text(path, content)
        .split("::")
        .map(str::trim)
        .collect::<Vec<_>>();
    let (module, name, crate_path) = match segments.as_slice() {
        [module, name] => (*module, *name, false),
        ["self", module, name] => (*module, *name, false),
        ["crate", module, name] => (*module, *name, true),
        _ => return None,
    };
    if !modules.contains(module) {
        return None;
    }
    Some((
        name,
        format!("{}::{module}", if crate_path { "crate" } else { "mod" }),
    ))
}
