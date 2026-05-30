use crate::graph::imports::common::extract_string_literal;
use std::collections::HashSet;
use tree_sitter::{Node, Tree};

pub(super) fn extract(tree: &Tree, content: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    visit(tree.root_node(), content, &mut result);
    result
}

fn is_candidate(path: &str) -> bool {
    path.starts_with('.')
        || path.starts_with('/')
        || path.starts_with('@')
        || path.starts_with("~/")
        || path.starts_with('#')
}

fn visit(node: Node<'_>, content: &str, result: &mut HashSet<String>) {
    match node.kind() {
        "import_statement" | "export_statement" => {
            if let Some(path) = module_source(node, content)
                && is_candidate(path)
            {
                result.insert(path.to_string());
            }
        }
        "call_expression" => {
            if let Some(path) = call_module_source(node, content)
                && is_candidate(path)
            {
                result.insert(path.to_string());
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
