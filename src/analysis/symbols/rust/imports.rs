use super::super::{ImportedSymbolFact, JavaScriptSymbolFacts, SymbolKind};
use super::text;
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(super) fn extract_uses(
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

fn extract_use(
    node: Node<'_>,
    content: &str,
    modules: &BTreeSet<&str>,
    facts: &mut JavaScriptSymbolFacts,
) {
    let Some(argument) = node.child_by_field_name("argument") else {
        return;
    };
    if matches!(argument.kind(), "scoped_use_list" | "use_list") {
        let mut imports = Vec::new();
        expand_grouped(argument, "", content, &mut imports);
        for (path, alias) in imports {
            record_grouped(&path, alias.as_deref(), node, modules, facts);
        }
        return;
    }
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
    record_scoped(path, node, alias, content, modules, facts);
}

fn expand_grouped(
    node: Node<'_>,
    prefix: &str,
    content: &str,
    imports: &mut Vec<(String, Option<String>)>,
) {
    match node.kind() {
        "scoped_use_list" => {
            let path = node
                .child_by_field_name("path")
                .map(|path| text(path, content))
                .unwrap_or("");
            let prefix = join(prefix, path);
            if let Some(list) = node.child_by_field_name("list") {
                expand_grouped(list, &prefix, content, imports);
            }
        }
        "use_list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                expand_grouped(child, prefix, content, imports);
            }
        }
        "use_as_clause" => {
            let Some(path) = node.child_by_field_name("path") else {
                return;
            };
            let alias = node
                .child_by_field_name("alias")
                .map(|alias| text(alias, content).to_string());
            imports.push((join(prefix, text(path, content)), alias));
        }
        "self" | "use_wildcard" => {}
        _ => imports.push((join(prefix, text(node, content)), None)),
    }
}

fn join(prefix: &str, path: &str) -> String {
    match (prefix.is_empty(), path.is_empty()) {
        (true, _) => path.to_string(),
        (_, true) => prefix.to_string(),
        _ => format!("{prefix}::{path}"),
    }
}

fn record_grouped(
    path: &str,
    alias: Option<&str>,
    span: Node<'_>,
    modules: &BTreeSet<&str>,
    facts: &mut JavaScriptSymbolFacts,
) {
    let segments = path.split("::").collect::<Vec<_>>();
    let (module, imported_name, crate_path) = match segments.as_slice() {
        [module, name] | ["self", module, name] => (*module, *name, false),
        ["crate", module, name] => (*module, *name, true),
        _ => return,
    };
    if modules.contains(module) {
        record_fact(
            imported_name,
            alias.unwrap_or(imported_name),
            format!("{}::{module}", if crate_path { "crate" } else { "mod" }),
            span,
            facts,
        );
    }
}

fn record_scoped(
    path: Node<'_>,
    span: Node<'_>,
    alias: Option<&str>,
    content: &str,
    modules: &BTreeSet<&str>,
    facts: &mut JavaScriptSymbolFacts,
) {
    let Some((name, module)) = super::paths::resolved_path(path, content, modules) else {
        return;
    };
    record_fact(name, alias.unwrap_or(name), module, span, facts);
}

fn record_fact(
    imported_name: &str,
    local_name: &str,
    module_specifier: String,
    span: Node<'_>,
    facts: &mut JavaScriptSymbolFacts,
) {
    facts.imports.push(ImportedSymbolFact {
        imported_name: imported_name.into(),
        local_name: local_name.into(),
        kind: SymbolKind::Value,
        module_specifier,
        line_start: span.start_position().row + 1,
        line_end: span.end_position().row + 1,
        byte_start: span.start_byte(),
        byte_end: span.end_byte(),
    });
}
