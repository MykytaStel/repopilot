use crate::analysis::parse::ParsedFile;
use std::collections::{BTreeMap, HashSet};
use tree_sitter::{Node, Tree};

mod guarded;
pub(super) use guarded::guarded_optional;

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

// Imports found only inside function bodies are deferred: they remain coupling
// edges but are excluded from module-load cycle detection. Module-scope imports
// and modules imported eagerly as well are not deferred.
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
    for module in dotted_viewset_modules(node, content) {
        scan.eager.entry(module).or_insert(span);
    }
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

/// Wagtail and compatible Django integrations can register a viewset on an
/// app config with a dotted class path instead of a Python import. The loader
/// resolves the module portion at runtime, so retain the full dotted path and
/// let the Python resolver choose the longest local module candidate.
fn dotted_viewset_modules(node: Node<'_>, content: &str) -> Vec<String> {
    if node.kind() != "assignment" {
        return Vec::new();
    }
    let Some(left) = node.child_by_field_name("left") else {
        return Vec::new();
    };
    let Some(right) = node.child_by_field_name("right") else {
        return Vec::new();
    };
    let Some(name) = left.utf8_text(content.as_bytes()).ok() else {
        return Vec::new();
    };
    if left.kind() != "identifier" || !name.ends_with("_viewset") || right.kind() != "string" {
        return Vec::new();
    }
    let Some(value) = right
        .utf8_text(content.as_bytes())
        .ok()
        .and_then(python_string_value)
    else {
        return Vec::new();
    };
    looks_like_dotted_class_path(value)
        .then(|| value.to_string())
        .into_iter()
        .collect()
}

fn python_string_value(raw: &str) -> Option<&str> {
    let raw = raw.trim();
    let quote = raw.chars().next()?;
    if !matches!(quote, '\'' | '"') || raw.chars().last()? != quote || raw.len() < 2 {
        return None;
    }
    Some(&raw[1..raw.len() - 1])
}

fn looks_like_dotted_class_path(value: &str) -> bool {
    let mut segments = value.split('.');
    let Some(last) = segments.next_back() else {
        return false;
    };
    last.chars()
        .next()
        .is_some_and(|character| character.is_uppercase())
        && segments.count() >= 2
        && value.split('.').all(valid_python_identifier)
}

fn valid_python_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::eager;
    use crate::analysis::parse::ParsedFile;

    fn imports(content: &str) -> Vec<String> {
        let parsed = ParsedFile::new(content, Some("Python"));
        let mut imports = eager(&parsed).into_iter().collect::<Vec<_>>();
        imports.sort();
        imports
    }

    #[test]
    fn captures_dotted_viewset_loader_references() {
        let content = concat!(
            "from django.apps import AppConfig\n",
            "\n",
            "class UsersConfig(AppConfig):\n",
            "    group_viewset = \"wagtail.users.views.groups.GroupViewSet\"\n",
        );

        assert!(imports(content).contains(&"wagtail.users.views.groups.GroupViewSet".to_string()));
    }

    #[test]
    fn does_not_treat_arbitrary_dotted_strings_as_imports() {
        let content = concat!(
            "class Settings:\n",
            "    template_name = \"wagtail.users.views.groups.GroupViewSet\"\n",
            "    viewset = \"wagtail.users.views.groups.GroupViewSet\"\n",
        );

        assert!(imports(content).is_empty());
    }
}
