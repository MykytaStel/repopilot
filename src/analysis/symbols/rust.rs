//! Bounded Rust facts: top-level `pub fn` and imports through a module declared
//! in the same file. Attributes, macro expansion and inline modules are opaque.
use super::{ExportedSymbolFact, ImportedSymbolFact, JavaScriptSymbolFacts, SymbolKind};
use std::collections::BTreeSet;
use tree_sitter::{Node, Tree};

pub(super) fn extract(content: &str, tree: &Tree) -> Option<JavaScriptSymbolFacts> {
    let root = tree.root_node();
    if root.has_error() || contains_opaque(root) {
        return None;
    }
    let mut facts = JavaScriptSymbolFacts::default();
    let mut modules = BTreeSet::new();
    let mut cursor = root.walk();
    for node in root.named_children(&mut cursor) {
        match node.kind() {
            "function_item" if public(node, content) => {
                let name = node.child_by_field_name("name")?;
                facts.exports.push(ExportedSymbolFact {
                    name: text(name, content).into(),
                    kind: SymbolKind::Value,
                    line_start: node.start_position().row + 1,
                    line_end: node.end_position().row + 1,
                });
            }
            "mod_item" => {
                if node.child_by_field_name("body").is_none() {
                    let name = text(node.child_by_field_name("name")?, content);
                    modules.insert(name);
                    facts.re_exports.push(name.into());
                }
            }
            // A forwarded name may replace a removed definition. We cannot
            // establish its origin, so withhold removal facts for this file.
            "use_declaration" if has_visibility(node) => return None,
            _ if node.child_by_field_name("name").is_some() => {
                // A preserved name with different visibility/item kind is not
                // function removal; do not infer type or visibility contracts.
                facts
                    .re_exports
                    .push(text(node.child_by_field_name("name")?, content).into());
            }
            _ => {}
        }
    }
    let mut cursor = root.walk();
    for node in root.named_children(&mut cursor) {
        if node.kind() == "use_declaration" {
            extract_use(node, content, &modules, &mut facts);
        }
    }
    extract_calls(root, content, &modules, &BTreeSet::new(), &mut facts);
    facts.exports.sort();
    facts.imports.sort();
    facts.imports.dedup();
    Some(facts)
}

fn public(node: Node<'_>, content: &str) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| child.kind() == "visibility_modifier" && text(child, content) == "pub")
}

fn has_visibility(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| child.kind() == "visibility_modifier")
}

fn contains_opaque(node: Node<'_>) -> bool {
    if matches!(
        node.kind(),
        "attribute_item"
            | "inner_attribute_item"
            | "macro_invocation"
            | "macro_definition"
            | "foreign_mod_item"
            | "extern_crate_declaration"
    ) || (node.kind() == "mod_item" && node.child_by_field_name("body").is_some())
    {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor).any(|child| {
        (node.kind() == "block" && matches!(child.kind(), "use_declaration" | "mod_item"))
            || contains_opaque(child)
    })
}

fn extract_use(
    node: Node<'_>,
    content: &str,
    modules: &BTreeSet<&str>,
    facts: &mut JavaScriptSymbolFacts,
) {
    let Some(argument) = node.child_by_field_name("argument") else {
        return;
    };
    let (path, alias) = if argument.kind() == "use_as_clause" {
        let Some(path) = argument.child_by_field_name("path") else {
            return;
        };
        let Some(alias) = argument.child_by_field_name("alias") else {
            return;
        };
        (path, Some(text(alias, content)))
    } else {
        (argument, None)
    };
    if path.kind() != "scoped_identifier" {
        return;
    }
    record(path, node, alias, content, modules, facts);
}

fn extract_calls(
    node: Node<'_>,
    content: &str,
    modules: &BTreeSet<&str>,
    inherited_shadowing: &BTreeSet<String>,
    facts: &mut JavaScriptSymbolFacts,
) {
    if matches!(node.kind(), "impl_item" | "trait_item")
        || (node.kind() == "function_item" && node.child_by_field_name("type_parameters").is_some())
    {
        return;
    }
    let mut shadowing = inherited_shadowing.clone();
    if node.kind() == "block" {
        collect_type_bindings(node, content, &mut shadowing);
    }
    if node.kind() == "call_expression"
        && let Some(function) = node.child_by_field_name("function")
        && function.kind() == "scoped_identifier"
    {
        let qualifier = text(function, content)
            .split("::")
            .next()
            .unwrap_or("")
            .trim();
        if !shadowing.contains(qualifier) {
            record(function, function, None, content, modules, facts);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_calls(child, content, modules, &shadowing, facts);
    }
}

fn collect_type_bindings(node: Node<'_>, content: &str, bindings: &mut BTreeSet<String>) {
    // Rust block items are in scope throughout their block, even before their
    // declaration. Only type-namespace items shadow a bare module qualifier;
    // self::/crate:: paths and neighboring blocks retain their module proof.
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if matches!(
            child.kind(),
            "struct_item" | "enum_item" | "union_item" | "type_item" | "trait_item"
        ) && let Some(name) = child.child_by_field_name("name")
        {
            bindings.insert(text(name, content).trim_start_matches("r#").into());
            bindings.insert(text(name, content).into());
        }
    }
}

fn record(
    path: Node<'_>,
    span: Node<'_>,
    alias: Option<&str>,
    content: &str,
    modules: &BTreeSet<&str>,
    facts: &mut JavaScriptSymbolFacts,
) {
    let segments = text(path, content)
        .split("::")
        .map(str::trim)
        .collect::<Vec<_>>();
    let (module, name, crate_path) = match segments.as_slice() {
        [module, name] => (*module, *name, false),
        ["self", module, name] => (*module, *name, false),
        ["crate", module, name] => (*module, *name, true),
        _ => return,
    };
    if !modules.contains(module) {
        return;
    }
    facts.imports.push(ImportedSymbolFact {
        imported_name: name.into(),
        local_name: alias.unwrap_or(name).into(),
        kind: SymbolKind::Value,
        module_specifier: format!("{}::{module}", if crate_path { "crate" } else { "mod" }),
        line_start: span.start_position().row + 1,
        line_end: span.end_position().row + 1,
        byte_start: span.start_byte(),
        byte_end: span.end_byte(),
    });
}

fn text<'a>(node: Node<'_>, content: &'a str) -> &'a str {
    &content[node.byte_range()]
}
