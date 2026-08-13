use super::{ExportedSymbolFact, ImportedSymbolFact, JavaScriptSymbolFacts, SymbolKind};
use crate::languages::import_support::extract_string_literal;
use crate::review::signals::content::ReviewSource;
use tree_sitter::Node;

pub(super) fn extract_javascript_symbol_facts(
    source: &ReviewSource,
) -> Option<JavaScriptSymbolFacts> {
    if !matches!(
        source.language_label(),
        Some("TypeScript" | "TypeScript React" | "JavaScript" | "JavaScript React")
    ) {
        return None;
    }
    let tree = source.tree()?;
    let root = tree.root_node();
    if root.has_error() {
        return None;
    }
    let mut facts = JavaScriptSymbolFacts::default();
    extract_program(root, source.content(), &mut facts);
    facts.exports.sort();
    facts.exports.dedup();
    facts.imports.sort();
    facts.imports.dedup();
    Some(facts)
}

fn extract_program(program: Node<'_>, content: &str, facts: &mut JavaScriptSymbolFacts) {
    let mut cursor = program.walk();
    for statement in program.named_children(&mut cursor) {
        match statement.kind() {
            "export_statement" => extract_export(statement, content, facts),
            "import_statement" => extract_import(statement, content, facts),
            _ => {}
        }
    }
}

fn extract_export(node: Node<'_>, content: &str, facts: &mut JavaScriptSymbolFacts) {
    if node.child_by_field_name("source").is_some() || has_direct_kind(node, "default") {
        return;
    }
    let statement_kind = if has_direct_type_modifier(node) {
        SymbolKind::Type
    } else {
        SymbolKind::Value
    };
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "export_clause" => extract_export_clause(child, content, statement_kind, facts),
            "lexical_declaration" | "variable_declaration" => {
                extract_variable_declarations(child, content, statement_kind, facts)
            }
            "function_declaration" | "class_declaration" | "enum_declaration" => {
                push_declaration(child, content, SymbolKind::Value, facts)
            }
            "type_alias_declaration" | "interface_declaration" => {
                push_declaration(child, content, SymbolKind::Type, facts)
            }
            _ => {}
        }
    }
}

fn extract_export_clause(
    clause: Node<'_>,
    content: &str,
    statement_kind: SymbolKind,
    facts: &mut JavaScriptSymbolFacts,
) {
    let mut cursor = clause.walk();
    for specifier in clause.named_children(&mut cursor) {
        if specifier.kind() != "export_specifier" {
            continue;
        }
        let local_name = semantic_field_text(specifier, "name", content);
        let name = semantic_field_text(specifier, "alias", content).or(local_name);
        if local_name == Some("default") || name == Some("default") {
            continue;
        }
        if let Some(name) = name {
            let (line_start, line_end) = span(specifier);
            facts.exports.push(ExportedSymbolFact {
                name: name.to_string(),
                kind: if has_direct_type_modifier(specifier) {
                    SymbolKind::Type
                } else {
                    statement_kind
                },
                line_start,
                line_end,
            });
        }
    }
}

fn extract_variable_declarations(
    declaration: Node<'_>,
    content: &str,
    kind: SymbolKind,
    facts: &mut JavaScriptSymbolFacts,
) {
    let mut cursor = declaration.walk();
    for declarator in declaration.named_children(&mut cursor) {
        if declarator.kind() == "variable_declarator" {
            push_declaration(declarator, content, kind, facts);
        }
    }
}

fn push_declaration(
    node: Node<'_>,
    content: &str,
    kind: SymbolKind,
    facts: &mut JavaScriptSymbolFacts,
) {
    if let Some(name) = field_text(node, "name", content) {
        let (line_start, line_end) = span(node);
        facts.exports.push(ExportedSymbolFact {
            name: name.to_string(),
            kind,
            line_start,
            line_end,
        });
    }
}

fn extract_import(node: Node<'_>, content: &str, facts: &mut JavaScriptSymbolFacts) {
    let Some(module_specifier) = node
        .child_by_field_name("source")
        .and_then(|source| source.utf8_text(content.as_bytes()).ok())
        .and_then(extract_string_literal)
    else {
        return;
    };
    let statement_kind = if has_direct_type_modifier(node) {
        SymbolKind::Type
    } else {
        SymbolKind::Value
    };
    if let Some(named_imports) = find_descendant(node, "named_imports") {
        extract_named_imports(
            named_imports,
            content,
            module_specifier,
            statement_kind,
            facts,
        );
    }
}

fn extract_named_imports(
    named_imports: Node<'_>,
    content: &str,
    module_specifier: &str,
    statement_kind: SymbolKind,
    facts: &mut JavaScriptSymbolFacts,
) {
    let mut cursor = named_imports.walk();
    for specifier in named_imports.named_children(&mut cursor) {
        if specifier.kind() != "import_specifier" {
            continue;
        }
        let Some(imported_name) = semantic_field_text(specifier, "name", content) else {
            continue;
        };
        if imported_name == "default" {
            continue;
        }
        let local_name = field_text(specifier, "alias", content).unwrap_or(imported_name);
        let (line_start, line_end) = span(specifier);
        facts.imports.push(ImportedSymbolFact {
            imported_name: imported_name.to_string(),
            local_name: local_name.to_string(),
            kind: if has_direct_type_modifier(specifier) {
                SymbolKind::Type
            } else {
                statement_kind
            },
            module_specifier: module_specifier.to_string(),
            line_start,
            line_end,
            byte_start: specifier.start_byte(),
            byte_end: specifier.end_byte(),
        });
    }
}

fn has_direct_type_modifier(node: Node<'_>) -> bool {
    has_direct_kind(node, "type") || has_direct_kind(node, "typeof")
}

fn has_direct_kind(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| child.kind() == kind)
}

fn find_descendant<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find_map(|child| find_descendant(child, kind))
}

fn field_text<'a>(node: Node<'_>, field: &str, content: &'a str) -> Option<&'a str> {
    node.child_by_field_name(field)?
        .utf8_text(content.as_bytes())
        .ok()
        .filter(|text| !text.is_empty())
}

fn semantic_field_text<'a>(node: Node<'_>, field: &str, content: &'a str) -> Option<&'a str> {
    let field = node.child_by_field_name(field)?;
    let text = field
        .utf8_text(content.as_bytes())
        .ok()
        .filter(|text| !text.is_empty())?;
    if field.kind() == "string" {
        extract_string_literal(text)
    } else {
        Some(text)
    }
}

fn span(node: Node<'_>) -> (usize, usize) {
    (node.start_position().row + 1, node.end_position().row + 1)
}
