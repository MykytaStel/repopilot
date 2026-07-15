use crate::analysis::parse::ParsedFile;
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

/// Imports deferred into a function body — the Python idiom for breaking a
/// load-time cycle; cycle detection subtracts them.
pub(super) fn deferred(parsed: &ParsedFile) -> HashSet<String> {
    parsed
        .tree()
        .map(|tree| extract_deferred(tree, parsed.content()))
        .unwrap_or_default()
}

fn extract_spans(tree: &Tree, content: &str) -> BTreeMap<String, (usize, usize)> {
    let mut scan = ImportScan::default();
    visit(tree.root_node(), content, false, &mut scan);
    // Eager spans win when a module is imported both ways (collected last).
    scan.deferred.into_iter().chain(scan.eager).collect()
}

fn extract(tree: &Tree, content: &str) -> HashSet<String> {
    extract_spans(tree, content).into_keys().collect()
}

/// Imports that appear *only* inside a `def`/method body. A function body runs
/// only when called, so such an import is a *deferred* import — the idiomatic
/// way Python breaks an import cycle, and not part of the module-load dependency
/// graph. The coupling graph still records these as edges (they are a real, if
/// lazy, dependency, which `dead-module`/`excessive-fan-out` must see), but
/// cycle detection subtracts them so a deferral does not resurrect the very
/// cycle it broke. A module imported *both* eagerly and lazily keeps its eager
/// edge, so it is excluded here. Module-scope imports inside `if`/`try` blocks
/// run at import time and are never deferred.
fn extract_deferred(tree: &Tree, content: &str) -> HashSet<String> {
    let mut scan = ImportScan::default();
    visit(tree.root_node(), content, false, &mut scan);
    scan.deferred
        .into_keys()
        .filter(|module| !scan.eager.contains_key(module))
        .collect()
}

#[derive(Default)]
struct ImportScan {
    eager: BTreeMap<String, (usize, usize)>,
    deferred: BTreeMap<String, (usize, usize)>,
}

fn visit(node: Node<'_>, content: &str, in_function: bool, scan: &mut ImportScan) {
    let span = (node.start_position().row + 1, node.end_position().row + 1);
    let modules = match node.kind() {
        "import_statement" => node
            .utf8_text(content.as_bytes())
            .ok()
            .map(import_statement_modules),
        "import_from_statement" => node
            .utf8_text(content.as_bytes())
            .ok()
            .map(from_import_modules),
        _ => None,
    };
    for module in modules.into_iter().flatten() {
        let bucket = if in_function {
            &mut scan.deferred
        } else {
            &mut scan.eager
        };
        bucket.entry(module).or_insert(span);
    }

    let child_in_function = in_function || node.kind() == "function_definition";
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, content, child_in_function, scan);
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
