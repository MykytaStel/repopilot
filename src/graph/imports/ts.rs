use crate::graph::imports::common::extract_string_literal;
use std::collections::{BTreeMap, HashSet};
use tree_sitter::{Node, Tree};

pub(super) fn extract_spans(tree: &Tree, content: &str) -> BTreeMap<String, (usize, usize)> {
    let mut result = BTreeMap::new();
    visit(tree.root_node(), content, &mut result);
    result
}

pub(super) fn extract(tree: &Tree, content: &str) -> HashSet<String> {
    extract_spans(tree, content).into_keys().collect()
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
