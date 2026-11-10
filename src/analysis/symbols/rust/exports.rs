use super::super::{
    ExportedSymbolFact, JavaScriptSymbolFacts, RustFunctionArityFact, RustStructFieldFact,
    SymbolKind,
};
use super::{Node, text};

pub(super) fn record_public_function(
    node: Node<'_>,
    content: &str,
    facts: &mut JavaScriptSymbolFacts,
) -> Option<()> {
    let name = node.child_by_field_name("name")?;
    let name: String = text(name, content).into();
    facts.exports.push(ExportedSymbolFact {
        name: name.clone(),
        kind: SymbolKind::Value,
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
    });
    if let Some(parameter_count) = function_arity(node) {
        facts
            .rust_contracts
            .as_mut()?
            .functions
            .push(RustFunctionArityFact {
                name,
                parameter_count,
            });
    }
    Some(())
}

pub(super) fn record_public_struct(
    node: Node<'_>,
    content: &str,
    facts: &mut JavaScriptSymbolFacts,
) -> Option<()> {
    let name = node.child_by_field_name("name")?;
    let struct_name = text(name, content).to_string();
    facts
        .rust_contracts
        .as_mut()?
        .public_structs
        .push(struct_name.clone());
    let Some(body) = node.child_by_field_name("body") else {
        return Some(());
    };
    if body.kind() != "field_declaration_list" {
        return Some(());
    }
    let mut cursor = body.walk();
    for field in body.named_children(&mut cursor) {
        if field.kind() == "field_declaration" && is_public(field, content) {
            let Some(name) = field.child_by_field_name("name") else {
                continue;
            };
            facts
                .rust_contracts
                .as_mut()?
                .public_struct_fields
                .push(RustStructFieldFact {
                    struct_name: struct_name.clone(),
                    field_name: text(name, content).into(),
                });
        }
    }
    Some(())
}

fn is_public(node: Node<'_>, content: &str) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| child.kind() == "visibility_modifier" && text(child, content) == "pub")
}

fn function_arity(node: Node<'_>) -> Option<usize> {
    if node.child_by_field_name("type_parameters").is_some() {
        return None;
    }
    let parameters = node.child_by_field_name("parameters")?;
    let mut cursor = parameters.walk();
    let children = parameters.named_children(&mut cursor).collect::<Vec<_>>();
    if children
        .iter()
        .any(|child| child.kind() == "variadic_parameter")
    {
        return None;
    }
    Some(children.len())
}
