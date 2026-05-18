use crate::graph::imports::common::{extract_string_literal, is_relative};
use std::collections::HashSet;
use tree_sitter::{Language, Node, Parser};

pub(super) fn extract(content: &str, language: Option<&str>) -> HashSet<String> {
    let Some(tree) = parse(content, language) else {
        return HashSet::new();
    };

    let mut result = HashSet::new();
    visit(tree.root_node(), content, &mut result);
    result
}

fn parse(content: &str, language_name: Option<&str>) -> Option<tree_sitter::Tree> {
    let language: Language = match language_name {
        Some("TypeScript React") => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Some("TypeScript") => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        _ => tree_sitter_javascript::LANGUAGE.into(),
    };
    let mut parser = Parser::new();
    parser.set_language(&language).ok()?;
    parser.parse(content, None)
}

fn visit(node: Node<'_>, content: &str, result: &mut HashSet<String>) {
    match node.kind() {
        "import_statement" | "export_statement" => {
            if let Some(path) = module_source(node, content)
                && is_relative(path)
            {
                result.insert(path.to_string());
            }
        }
        "call_expression" => {
            if let Some(path) = require_source(node, content)
                && is_relative(path)
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

fn require_source<'a>(node: Node<'_>, content: &'a str) -> Option<&'a str> {
    let function = node.child_by_field_name("function")?;
    if function.utf8_text(content.as_bytes()).ok()? != "require" {
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
