//! Local same-package type references for Java and Kotlin import graphs.

use crate::analysis::parse::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(crate) fn same_package_references(parsed: &ParsedFile<'_>) -> BTreeMap<String, (usize, usize)> {
    let Some(tree) = parsed.tree() else {
        return BTreeMap::new();
    };
    if tree.root_node().has_error() {
        return BTreeMap::new();
    }
    let Some(package) = package_name(parsed.content(), tree.root_node()) else {
        return BTreeMap::new();
    };

    let mut references = BTreeMap::new();
    collect_type_references(
        parsed.content(),
        tree.root_node(),
        &package,
        &mut references,
    );
    references
}

fn package_name(content: &str, root: Node<'_>) -> Option<String> {
    let mut cursor = root.walk();
    root.named_children(&mut cursor).find_map(|node| {
        if !matches!(node.kind(), "package_declaration" | "package_header") {
            return None;
        }
        let text = node.utf8_text(content.as_bytes()).ok()?;
        let package = text.strip_prefix("package")?.trim_end_matches(';').trim();
        (!package.is_empty()
            && package
                .split('.')
                .all(|segment| !segment.is_empty() && segment.chars().all(is_package_char)))
        .then(|| package.to_string())
    })
}

fn is_package_char(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn collect_type_references(
    content: &str,
    node: Node<'_>,
    package: &str,
    references: &mut BTreeMap<String, (usize, usize)>,
) {
    if !inside_import(node)
        && !inside_qualified_type(node)
        && let Some(reference) = referenced_type_name(node)
        && let Ok(name) = reference.utf8_text(content.as_bytes())
    {
        let name = name.trim_start_matches('`');
        let name = name.trim_end_matches('`');
        if !name.is_empty() {
            references
                .entry(format!("{package}.{name}"))
                .or_insert((node.start_position().row + 1, node.end_position().row + 1));
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_type_references(content, child, package, references);
    }
}

fn referenced_type_name(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "type_identifier" && !is_declaration_name(node) {
        return Some(node);
    }
    if node.kind() != "user_type" {
        return None;
    }
    let mut cursor = node.walk();
    let identifiers = node
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "identifier")
        .collect::<Vec<_>>();
    (identifiers.len() == 1).then(|| identifiers[0])
}

fn is_declaration_name(node: Node<'_>) -> bool {
    node.parent()
        .and_then(|parent| parent.child_by_field_name("name"))
        .is_some_and(|name| {
            name.start_byte() == node.start_byte() && name.end_byte() == node.end_byte()
        })
}

fn inside_import(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if ancestor.kind().contains("import") {
            return true;
        }
        parent = ancestor.parent();
    }
    false
}

fn inside_qualified_type(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if ancestor.kind() == "scoped_type_identifier" {
            return true;
        }
        parent = ancestor.parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::same_package_references;
    use crate::analysis::parse::ParsedFile;

    #[test]
    fn java_records_only_unqualified_same_package_type_uses() {
        let parsed = ParsedFile::new(
            "package app.core;\nclass Main {\n  Local value;\n  other.Remote qualified;\n  // Fake comment type\n}\n",
            Some("Java"),
        );

        let references = same_package_references(&parsed);

        assert_eq!(references.get("app.core.Local"), Some(&(3, 3)));
        assert!(!references.contains_key("app.core.Remote"));
        assert!(!references.contains_key("app.core.Fake"));
    }

    #[test]
    fn kotlin_records_simple_user_types_and_ignores_qualified_types() {
        let parsed = ParsedFile::new(
            "package app.core\nclass Main(val local: Local, val qualified: other.Remote)\n",
            Some("Kotlin"),
        );

        let references = same_package_references(&parsed);

        assert!(references.contains_key("app.core.Local"));
        assert!(!references.contains_key("app.core.Remote"));
    }

    #[test]
    fn malformed_source_does_not_create_same_package_edges() {
        let parsed = ParsedFile::new(
            "package app.core;\nclass Main { Local value = ; }\n",
            Some("Java"),
        );

        assert!(same_package_references(&parsed).is_empty());
    }
}
