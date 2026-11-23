use super::ImportedHelper;
use crate::review::signals::content::ReviewSource;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub(super) fn imports(source: &ReviewSource) -> Vec<ImportedHelper> {
    let Some(tree) = source.tree() else {
        return Vec::new();
    };
    let root = tree.root_node();
    let bytes = source.content().as_bytes();
    let bindings = scope_bindings(root, bytes);
    if bindings.contains_key("*") {
        return Vec::new();
    }
    let mut cursor = root.walk();
    let mut imports = root
        .named_children(&mut cursor)
        .filter(|node| node.kind() == "import_from_statement")
        .flat_map(|node| named_imports(node, bytes))
        .filter(|import| bindings.get(import.local_name.as_str()) == Some(&1))
        .collect::<Vec<_>>();
    imports.sort_by(|left, right| {
        left.module_specifier
            .cmp(&right.module_specifier)
            .then(left.imported_name.cmp(&right.imported_name))
            .then(left.local_name.cmp(&right.local_name))
    });
    imports
}

pub(in super::super) fn shadowed_in_test(source: &ReviewSource, start: usize) -> BTreeSet<&str> {
    let Some(tree) = source.tree() else {
        return BTreeSet::from(["*"]);
    };
    let Some(mut node) = tree.root_node().descendant_for_byte_range(start, start + 1) else {
        return BTreeSet::from(["*"]);
    };
    let mut names = BTreeSet::new();
    loop {
        if node.kind() == "function_definition" {
            names.extend(scope_bindings(node, source.content().as_bytes()).into_keys());
        }
        let Some(parent) = node.parent() else {
            return names;
        };
        node = parent;
    }
}

fn named_imports(node: Node<'_>, content: &[u8]) -> Vec<ImportedHelper> {
    let Some(module) = node
        .child_by_field_name("module_name")
        .and_then(|module| module.utf8_text(content).ok())
    else {
        return Vec::new();
    };
    let mut cursor = node.walk();
    node.children_by_field_name("name", &mut cursor)
        .filter_map(|name| {
            let (imported, local) = import_names(name, content)?;
            Some(ImportedHelper {
                module_specifier: module.into(),
                imported_name: imported.into(),
                local_name: local.into(),
            })
        })
        .collect()
}

fn import_names<'a>(name: Node<'_>, content: &'a [u8]) -> Option<(&'a str, &'a str)> {
    if name.kind() == "aliased_import" {
        let imported = name.child_by_field_name("name")?.utf8_text(content).ok()?;
        let local = name.child_by_field_name("alias")?.utf8_text(content).ok()?;
        Some((imported, local))
    } else {
        let name = name.utf8_text(content).ok()?;
        Some((name, name.split('.').next()?))
    }
}

fn scope_bindings<'a>(root: Node<'_>, content: &'a [u8]) -> BTreeMap<&'a str, usize> {
    let mut names = BTreeMap::new();
    if let Some(parameters) = root.child_by_field_name("parameters") {
        collect_pattern(parameters, content, &mut names);
    }
    let body = root.child_by_field_name("body").unwrap_or(root);
    collect_bindings(body, content, &mut names);
    names
}

fn collect_bindings<'a>(node: Node<'_>, content: &'a [u8], names: &mut BTreeMap<&'a str, usize>) {
    let field = match node.kind() {
        "function_definition" | "class_definition" => {
            if let Some(name) = node.child_by_field_name("name") {
                collect_pattern(name, content, names);
            }
            return;
        }
        "lambda" => return,
        "import_statement" | "import_from_statement" => {
            let mut cursor = node.walk();
            for name in node.children_by_field_name("name", &mut cursor) {
                if let Some((_, local)) = import_names(name, content) {
                    *names.entry(local).or_default() += 1;
                }
            }
            if node
                .named_children(&mut cursor)
                .any(|child| child.kind() == "wildcard_import")
            {
                names.insert("*", 1);
            }
            return;
        }
        "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => Some("left"),
        "named_expression" => Some("name"),
        "as_pattern" | "except_clause" => Some("alias"),
        "delete_statement" | "case_pattern" | "global_statement" | "nonlocal_statement" => {
            collect_pattern(node, content, names);
            return;
        }
        _ => None,
    };
    let binding = field.and_then(|name| node.child_by_field_name(name));
    if let Some(pattern) = binding {
        collect_pattern(pattern, content, names);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if Some(child) != binding {
            collect_bindings(child, content, names);
        }
    }
}

fn collect_pattern<'a>(node: Node<'_>, content: &'a [u8], names: &mut BTreeMap<&'a str, usize>) {
    match node.kind() {
        "identifier" => {
            if let Ok(name) = node.utf8_text(content) {
                *names.entry(name).or_default() += 1;
            }
        }
        "attribute" | "subscript" | "type" => {}
        "default_parameter" | "typed_default_parameter" => {
            if let Some(name) = node.child_by_field_name("name") {
                collect_pattern(name, content, names);
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_pattern(child, content, names);
            }
        }
    }
}
