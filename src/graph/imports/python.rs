use std::cell::RefCell;
use std::collections::HashSet;
use tree_sitter::{Node, Parser};

thread_local! {
    static PYTHON_PARSER: RefCell<Parser> = RefCell::new({
        let mut p = Parser::new();
        p.set_language(&tree_sitter_python::LANGUAGE.into())
            .expect("tree-sitter-python grammar should load");
        p
    });
}

pub(super) fn extract(content: &str) -> HashSet<String> {
    let tree = PYTHON_PARSER.with(|cell| {
        let mut p = cell.borrow_mut();
        p.reset();
        p.parse(content, None)
    });
    let Some(tree) = tree else {
        return HashSet::new();
    };

    let mut result = HashSet::new();
    visit(tree.root_node(), content, &mut result);
    result
}

fn visit(node: Node<'_>, content: &str, result: &mut HashSet<String>) {
    if node.kind() == "import_from_statement"
        && let Some(module) = relative_from_module(node, content)
    {
        result.insert(module);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, content, result);
    }
}

fn relative_from_module(node: Node<'_>, content: &str) -> Option<String> {
    let text = node.utf8_text(content.as_bytes()).ok()?;
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let rest = normalized.strip_prefix("from ")?;
    let import_pos = rest.find(" import ")?;
    let module = rest[..import_pos].trim();
    module.starts_with('.').then(|| module.to_string())
}
