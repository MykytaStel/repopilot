use crate::graph::imports::common::extract_string_literal;
use std::cell::RefCell;
use std::collections::HashSet;
use tree_sitter::{Node, Parser};

thread_local! {
    static TS_PARSER: RefCell<Parser> = RefCell::new({
        let mut p = Parser::new();
        p.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .expect("tree-sitter-typescript grammar should load");
        p
    });
    static TSX_PARSER: RefCell<Parser> = RefCell::new({
        let mut p = Parser::new();
        p.set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
            .expect("tree-sitter-tsx grammar should load");
        p
    });
    static JS_PARSER: RefCell<Parser> = RefCell::new({
        let mut p = Parser::new();
        p.set_language(&tree_sitter_javascript::LANGUAGE.into())
            .expect("tree-sitter-javascript grammar should load");
        p
    });
}

pub(super) fn extract(content: &str, language: Option<&str>) -> HashSet<String> {
    let tree = match language {
        Some("TypeScript React") => TSX_PARSER.with(|cell| {
            let mut p = cell.borrow_mut();
            p.reset();
            p.parse(content, None)
        }),
        Some("TypeScript") => TS_PARSER.with(|cell| {
            let mut p = cell.borrow_mut();
            p.reset();
            p.parse(content, None)
        }),
        _ => JS_PARSER.with(|cell| {
            let mut p = cell.borrow_mut();
            p.reset();
            p.parse(content, None)
        }),
    };

    let Some(tree) = tree else {
        return HashSet::new();
    };

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
