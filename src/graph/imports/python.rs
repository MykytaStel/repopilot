use std::collections::HashSet;
use tree_sitter::{Node, Tree};

pub(super) fn extract(tree: &Tree, content: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    visit(tree.root_node(), content, &mut result);
    result
}

fn visit(node: Node<'_>, content: &str, result: &mut HashSet<String>) {
    match node.kind() {
        "import_statement" => {
            if let Ok(text) = node.utf8_text(content.as_bytes()) {
                result.extend(import_statement_modules(text));
            }
        }
        "import_from_statement" => {
            if let Ok(text) = node.utf8_text(content.as_bytes()) {
                result.extend(from_import_modules(text));
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, content, result);
    }
}

fn import_statement_modules(text: &str) -> Vec<String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let Some(rest) = normalized.strip_prefix("import ") else {
        return Vec::new();
    };

    split_import_names(rest)
        .into_iter()
        .filter(|module| !module.is_empty())
        .collect()
}

fn from_import_modules(text: &str) -> Vec<String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let Some(rest) = normalized.strip_prefix("from ") else {
        return Vec::new();
    };
    let Some(import_pos) = rest.find(" import ") else {
        return Vec::new();
    };

    let module = rest[..import_pos].trim();
    if module.is_empty() {
        return Vec::new();
    }

    let mut modules = vec![module.to_string()];
    let imported = rest[import_pos + " import ".len()..].trim();
    modules.extend(imported_submodules(module, imported));
    modules
}

fn imported_submodules(module: &str, imported: &str) -> Vec<String> {
    split_import_names(imported)
        .into_iter()
        .filter(|name| looks_like_python_module_name(name))
        .map(|name| join_python_module(module, &name))
        .collect()
}

fn split_import_names(input: &str) -> Vec<String> {
    input
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split(',')
        .filter_map(|part| {
            let name = part
                .trim()
                .split(" as ")
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches('(')
                .trim_end_matches(')');
            (!name.is_empty() && name != "*").then(|| name.to_string())
        })
        .collect()
}

fn looks_like_python_module_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_lowercase())
        && name
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn join_python_module(module: &str, name: &str) -> String {
    if module.chars().all(|character| character == '.') {
        format!("{module}{name}")
    } else {
        format!("{module}.{name}")
    }
}
