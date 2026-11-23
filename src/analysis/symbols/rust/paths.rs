use super::super::{
    ImportedSymbolFact, JavaScriptSymbolFacts, RustCallFact, RustEnumVariantUseFact,
    RustFieldUseFact,
};
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

pub(super) fn record_enum_variant_uses(
    node: Node<'_>,
    content: &str,
    modules: &BTreeSet<&str>,
    shadowing: &BTreeSet<String>,
    facts: &mut JavaScriptSymbolFacts,
) {
    if is_import_path(node) || !is_enum_variant_path(node) {
        return;
    }
    let Some((enum_name, variant_name, module_specifier)) =
        resolved_enum_variant_path(node, content, modules, shadowing, &facts.imports)
    else {
        return;
    };
    let Some(contracts) = facts.rust_contracts.as_mut() else {
        return;
    };
    contracts.enum_variant_uses.push(RustEnumVariantUseFact {
        enum_name,
        variant_name,
        module_specifier,
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        byte_start: node.start_byte(),
        byte_end: node.end_byte(),
    });
}

fn is_enum_variant_path(node: Node<'_>) -> bool {
    match node.kind() {
        "scoped_identifier" => !has_scoped_path_parent(node),
        "scoped_type_identifier" => node
            .parent()
            .is_some_and(|parent| matches!(parent.kind(), "struct_pattern" | "struct_expression")),
        _ => false,
    }
}

fn has_scoped_path_parent(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        matches!(
            parent.kind(),
            "scoped_identifier" | "scoped_type_identifier"
        )
    })
}

fn is_import_path(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(candidate) = parent {
        if candidate.kind() == "use_declaration" {
            return true;
        }
        parent = candidate.parent();
    }
    false
}

fn resolved_enum_variant_path(
    path: Node<'_>,
    content: &str,
    modules: &BTreeSet<&str>,
    shadowing: &BTreeSet<String>,
    imports: &[ImportedSymbolFact],
) -> Option<(String, String, String)> {
    let segments = text(path, content)
        .split("::")
        .map(str::trim)
        .collect::<Vec<_>>();
    let first = segments.first()?.trim_start_matches("r#");
    if !matches!(first, "self" | "crate") && shadowing.contains(first) {
        return None;
    }
    let variant = segments.last()?.to_string();
    if let Some((start, root)) = module_path_start(&segments, modules, 2) {
        let module_path = segments[start..segments.len() - 2].join("::");
        return Some((
            segments[segments.len() - 2].to_string(),
            variant,
            format!("{root}::{module_path}"),
        ));
    }
    if segments.len() != 2 {
        return None;
    }
    let local_name = segments[0];
    let matching = imports
        .iter()
        .filter(|import| import.local_name == local_name)
        .collect::<Vec<_>>();
    let [import] = matching.as_slice() else {
        return None;
    };
    Some((
        import.imported_name.clone(),
        variant,
        import.module_specifier.clone(),
    ))
}

pub(super) fn resolved_text_path<'a>(
    raw: &'a str,
    modules: &BTreeSet<&str>,
) -> Option<(&'a str, String)> {
    let segments = raw.split("::").map(str::trim).collect::<Vec<_>>();
    let (start, root) = module_path_start(&segments, modules, 1)?;
    let module_path = segments[start..segments.len() - 1].join("::");
    Some((
        segments[segments.len() - 1],
        format!("{root}::{module_path}"),
    ))
}

fn module_path_start(
    segments: &[&str],
    modules: &BTreeSet<&str>,
    trailing_items: usize,
) -> Option<(usize, &'static str)> {
    let (start, root) = match segments.first().copied()? {
        "crate" => (1, "crate"),
        "self" => (1, "mod"),
        _ => (0, "mod"),
    };
    let module_end = segments.len().checked_sub(trailing_items)?;
    (module_end > start && modules.contains(segments[start])).then_some((start, root))
}

pub(super) fn resolved_path<'a>(
    path: Node<'_>,
    content: &'a str,
    modules: &BTreeSet<&str>,
) -> Option<(&'a str, String)> {
    resolved_text_path(text(path, content), modules)
}
