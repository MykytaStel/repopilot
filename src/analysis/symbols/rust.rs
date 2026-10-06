//! Bounded Rust facts: top-level `pub fn` and imports through a module declared
//! in the same file. Attributes, macro expansion and inline modules are opaque.
mod exports;
mod paths;

use super::{ImportedSymbolFact, JavaScriptSymbolFacts, RustContractFacts, SymbolKind};
use exports::record_public_function;
use paths::{record_call, resolved_path};
use std::collections::BTreeSet;
use tree_sitter::{Node, Tree};

pub(super) fn extract(content: &str, tree: &Tree) -> Option<JavaScriptSymbolFacts> {
    let root = tree.root_node();
    if root.has_error() || contains_opaque(root) {
        return None;
    }
    let mut facts = JavaScriptSymbolFacts {
        rust_contracts: Some(RustContractFacts::default()),
        ..JavaScriptSymbolFacts::default()
    };
    let modules = top_level_facts(root, content, &mut facts)?;
    extract_uses(root, content, &modules, &mut facts);
    extract_calls(root, content, &modules, &BTreeSet::new(), &mut facts);
    sort_facts(&mut facts);
    Some(facts)
}

fn top_level_facts<'a>(
    root: Node<'_>,
    content: &'a str,
    facts: &mut JavaScriptSymbolFacts,
) -> Option<BTreeSet<&'a str>> {
    let mut modules = BTreeSet::new();
    let mut cursor = root.walk();
    for node in root.named_children(&mut cursor) {
        collect_top_level_node(node, content, &mut modules, facts)?;
    }
    Some(modules)
}

fn collect_top_level_node<'a>(
    node: Node<'_>,
    content: &'a str,
    modules: &mut BTreeSet<&'a str>,
    facts: &mut JavaScriptSymbolFacts,
) -> Option<()> {
    if node.kind() == "function_item" && public(node, content) {
        record_public_function(node, content, facts)?;
        return Some(());
    }
    if node.kind() == "mod_item" && node.child_by_field_name("body").is_none() {
        let name = text(node.child_by_field_name("name")?, content);
        modules.insert(name);
        facts.re_exports.push(name.into());
        return Some(());
    }
    // A forwarded name may replace a removed definition. We cannot establish
    // its origin, so withhold removal facts for this file.
    if node.kind() == "use_declaration" && has_visibility(node) {
        return None;
    }
    // A preserved name with different visibility/item kind is not function
    // removal; do not infer type or visibility contracts.
    if let Some(name) = node.child_by_field_name("name") {
        facts.re_exports.push(text(name, content).into());
    }
    Some(())
}

fn extract_uses(
    root: Node<'_>,
    content: &str,
    modules: &BTreeSet<&str>,
    facts: &mut JavaScriptSymbolFacts,
) {
    let mut cursor = root.walk();
    for node in root.named_children(&mut cursor) {
        if node.kind() == "use_declaration" {
            extract_use(node, content, modules, facts);
        }
    }
}

fn sort_facts(facts: &mut JavaScriptSymbolFacts) {
    facts.exports.sort();
    facts.imports.sort();
    facts.imports.dedup();
    if let Some(contracts) = facts.rust_contracts.as_mut() {
        contracts.functions.sort();
        contracts.calls.sort();
        contracts.calls.dedup();
    }
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

fn record(
    path: Node<'_>,
    span: Node<'_>,
    alias: Option<&str>,
    content: &str,
    modules: &BTreeSet<&str>,
    facts: &mut JavaScriptSymbolFacts,
) {
    let Some((name, module_specifier)) = resolved_path(path, content, modules) else {
        return;
    };
    facts.imports.push(ImportedSymbolFact {
        imported_name: name.into(),
        local_name: alias.unwrap_or(name).into(),
        kind: SymbolKind::Value,
        module_specifier,
        line_start: span.start_position().row + 1,
        line_end: span.end_position().row + 1,
        byte_start: span.start_byte(),
        byte_end: span.end_byte(),
    });
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
        record_qualified_call(node, function, content, modules, &shadowing, facts);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract_calls(child, content, modules, &shadowing, facts);
    }
}

fn record_qualified_call(
    call: Node<'_>,
    function: Node<'_>,
    content: &str,
    modules: &BTreeSet<&str>,
    shadowing: &BTreeSet<String>,
    facts: &mut JavaScriptSymbolFacts,
) {
    let qualifier = text(function, content)
        .split("::")
        .next()
        .unwrap_or("")
        .trim();
    if shadowing.contains(qualifier) {
        return;
    }
    record(function, function, None, content, modules, facts);
    if let Some(arguments) = call.child_by_field_name("arguments") {
        record_call(function, arguments, content, modules, facts);
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

pub(super) fn text<'a>(node: Node<'_>, content: &'a str) -> &'a str {
    &content[node.byte_range()]
}
