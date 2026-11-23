use super::super::{
    ExportedSymbolFact, JavaScriptSymbolFacts, RustEnumVariantFact, RustFunctionArityFact,
    RustStructFieldFact, SymbolKind,
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

pub(super) fn record_public_enum(
    node: Node<'_>,
    content: &str,
    facts: &mut JavaScriptSymbolFacts,
) -> Option<()> {
    let name: String = text(node.child_by_field_name("name")?, content).into();
    let contracts = facts.rust_contracts.as_mut()?;
    contracts.public_enums.push(name.clone());
    let Some(body) = node.child_by_field_name("body") else {
        return Some(());
    };
    if body.kind() != "enum_variant_list" {
        return Some(());
    }
    let mut cursor = body.walk();
    for variant in body.named_children(&mut cursor) {
        if variant.kind() != "enum_variant" {
            continue;
        }
        let Some(variant_name) = variant.child_by_field_name("name") else {
            continue;
        };
        contracts.public_enum_variants.push(RustEnumVariantFact {
            enum_name: name.clone(),
            variant_name: text(variant_name, content).into(),
            line_start: variant.start_position().row + 1,
            line_end: variant.end_position().row + 1,
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

pub(super) fn record_inherent_items(
    node: Node<'_>,
    content: &str,
    facts: &mut JavaScriptSymbolFacts,
) {
    if node.child_by_field_name("trait").is_some() {
        return;
    }
    let (Some(ty), Some(body), Some(contracts)) = (
        node.child_by_field_name("type"),
        node.child_by_field_name("body"),
        facts.rust_contracts.as_mut(),
    ) else {
        return;
    };
    if ty.kind() != "type_identifier" {
        return;
    }
    let ty = text(ty, content).trim_start_matches("r#");
    let mut cursor = body.walk();
    for item in body.named_children(&mut cursor) {
        if matches!(item.kind(), "function_item" | "const_item")
            && let Some(name) = item.child_by_field_name("name")
        {
            contracts.inherent_items.push((
                ty.into(),
                text(name, content).trim_start_matches("r#").into(),
            ));
        }
    }
}
