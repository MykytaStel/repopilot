use crate::analysis::parse::ParsedFile;
use crate::languages::import_support::extract_string_literal;
use std::collections::{BTreeMap, HashSet};
use tree_sitter::{Node, Tree};

pub(super) fn eager(parsed: &ParsedFile) -> HashSet<String> {
    parsed
        .tree()
        .map(|tree| extract(tree, parsed.content()))
        .unwrap_or_default()
}

pub(super) fn spans(parsed: &ParsedFile) -> BTreeMap<String, (usize, usize)> {
    parsed
        .tree()
        .map(|tree| extract_spans(tree, parsed.content()))
        .unwrap_or_default()
}

/// Type-only imports (`import type`, `export type`) — erased by the
/// compiler, so cycle detection subtracts them.
pub(super) fn deferred(parsed: &ParsedFile) -> HashSet<String> {
    parsed
        .tree()
        .map(|tree| extract_type_only(tree, parsed.content()))
        .unwrap_or_default()
}

fn extract_spans(tree: &Tree, content: &str) -> BTreeMap<String, (usize, usize)> {
    let mut result = BTreeMap::new();
    visit(tree.root_node(), content, &mut result);
    result
}

fn extract(tree: &Tree, content: &str) -> HashSet<String> {
    extract_spans(tree, content).into_keys().collect()
}

/// Module paths imported *type-only* — `import type { … }`, `export type { … }`,
/// or named imports/exports whose every specifier is inline type-only
/// (`import { type A, type B }`) — and never imported as a value. The TypeScript
/// compiler erases these, so they create no runtime edge and must be subtracted
/// from cycle detection (they stay real edges for coupling/fan-out, like Python
/// deferred imports). A module that is *also* imported as a value anywhere —
/// including a mixed `import { type A, B }`, a default/namespace import, or a
/// dynamic `import("…")` — is a runtime edge and is excluded here.
fn extract_type_only(tree: &Tree, content: &str) -> HashSet<String> {
    let mut type_only: HashSet<String> = HashSet::new();
    let mut value: HashSet<String> = HashSet::new();
    collect_type_only(tree.root_node(), content, &mut type_only, &mut value);
    type_only.retain(|path| !value.contains(path));
    type_only
}

fn collect_type_only(
    node: Node<'_>,
    content: &str,
    type_only: &mut HashSet<String>,
    value: &mut HashSet<String>,
) {
    match node.kind() {
        "import_statement" | "export_statement" => {
            if let Some(path) = module_source(node, content).filter(|path| is_candidate(path)) {
                if statement_is_type_only(node, content) {
                    type_only.insert(path.to_string());
                } else {
                    value.insert(path.to_string());
                }
            }
        }
        "call_expression" => {
            // A dynamic `import("…")` / `require("…")` executes the module.
            if let Some(path) = call_module_source(node, content).filter(|path| is_candidate(path))
            {
                value.insert(path.to_string());
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_type_only(child, content, type_only, value);
    }
}

fn statement_is_type_only(node: Node<'_>, content: &str) -> bool {
    if has_direct_type_modifier(node) {
        return true;
    }

    starts_with_named_specifiers(node, content) && all_named_specifiers_are_type_only(node)
}

fn has_direct_type_modifier(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| matches!(child.kind(), "type" | "typeof"))
}

fn starts_with_named_specifiers(node: Node<'_>, content: &str) -> bool {
    let Ok(text) = node.utf8_text(content.as_bytes()) else {
        return false;
    };
    let keyword = match node.kind() {
        "import_statement" => "import",
        "export_statement" => "export",
        _ => return false,
    };
    text.trim_start()
        .strip_prefix(keyword)
        .is_some_and(|rest| rest.trim_start().starts_with('{'))
}

fn all_named_specifiers_are_type_only(node: Node<'_>) -> bool {
    let mut specifier_count = 0;
    let mut all_type_only = true;
    visit_named_specifiers(node, &mut |specifier| {
        specifier_count += 1;
        all_type_only &= has_direct_type_modifier(specifier);
    });
    specifier_count > 0 && all_type_only
}

fn visit_named_specifiers(node: Node<'_>, visit: &mut impl FnMut(Node<'_>)) {
    if matches!(node.kind(), "import_specifier" | "export_specifier") {
        visit(node);
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_named_specifiers(child, visit);
    }
}

fn is_candidate(path: &str) -> bool {
    path.starts_with('.')
        || path.starts_with('/')
        || path.starts_with('@')
        || path.starts_with("~/")
        || path.starts_with('#')
}

fn visit(node: Node<'_>, content: &str, result: &mut BTreeMap<String, (usize, usize)>) {
    let span = (node.start_position().row + 1, node.end_position().row + 1);
    match node.kind() {
        "import_statement" | "export_statement" => {
            if let Some(path) = module_source(node, content)
                && is_candidate(path)
            {
                result.entry(path.to_string()).or_insert(span);
            }
        }
        "call_expression" => {
            if let Some(path) = call_module_source(node, content)
                && is_candidate(path)
            {
                result.entry(path.to_string()).or_insert(span);
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, content, result);
    }
}

fn module_source<'a>(node: Node<'_>, content: &'a str) -> Option<&'a str> {
    if let Some(source) = node.child_by_field_name("source") {
        let text = source.utf8_text(content.as_bytes()).ok()?.trim();
        return extract_string_literal(text);
    }

    let text = node.utf8_text(content.as_bytes()).ok()?;
    extract_from_path(text)
}

fn call_module_source<'a>(node: Node<'_>, content: &'a str) -> Option<&'a str> {
    let function = node.child_by_field_name("function")?;
    let function = function.utf8_text(content.as_bytes()).ok()?;
    if function != "require" && function != "import" {
        return None;
    }
    let arguments = node.child_by_field_name("arguments")?;
    let text = arguments.utf8_text(content.as_bytes()).ok()?.trim();
    let text = text.strip_prefix('(')?.trim();
    let text = text.strip_suffix(')')?.trim();
    extract_string_literal(text)
}

fn extract_from_path(line: &str) -> Option<&str> {
    let pos = line.rfind(" from ")?;
    let after = line[pos + 6..].trim();
    extract_string_literal(after)
}
