use super::field_text;
use super::{ExportedSymbolFact, ImportedSymbolFact, JavaScriptSymbolFacts, SymbolKind};
use tree_sitter::Node;

// The reserved exported name `default` is distinct from the declaration's
// local name. Imported bindings and forwarding are deliberately not ownership proof.
pub(super) fn extract_export(node: Node<'_>, content: &str, facts: &mut JavaScriptSymbolFacts) {
    let mut value = node.child_by_field_name("value");
    while let Some(wrapper) = value.filter(|value| value.kind() == "parenthesized_expression") {
        value = wrapper.named_child(0);
    }
    if let Some(value) = value.filter(|value| value.kind() == "identifier") {
        let name = value.utf8_text(content.as_bytes()).ok();
        if !is_local_binding(node, name, content) {
            facts.re_exports.push("default".to_string());
            return;
        }
    }
    let (line_start, line_end) = span(node);
    let kind = node
        .child_by_field_name("declaration")
        .filter(|declaration| {
            matches!(
                declaration.kind(),
                "interface_declaration" | "type_alias_declaration"
            )
        })
        .map_or(SymbolKind::Value, |_| SymbolKind::Type);
    facts.exports.push(ExportedSymbolFact {
        name: "default".to_string(),
        kind,
        line_start,
        line_end,
    });
}

pub(super) fn is_local_binding(node: Node<'_>, name: Option<&str>, content: &str) -> bool {
    let Some(name) = name else {
        return false;
    };
    let mut program = node;
    while let Some(parent) = program.parent() {
        program = parent;
    }
    let mut cursor = program.walk();
    program.named_children(&mut cursor).any(|statement| {
        let declaration = statement
            .child_by_field_name("declaration")
            .unwrap_or(statement);
        match declaration.kind() {
            "function_declaration" | "class_declaration" | "enum_declaration" => {
                field_text(declaration, "name", content) == Some(name)
            }
            "lexical_declaration" | "variable_declaration" => {
                let mut cursor = declaration.walk();
                declaration
                    .named_children(&mut cursor)
                    .any(|binding| field_text(binding, "name", content) == Some(name))
            }
            _ => false,
        }
    })
}

pub(super) fn extract_import(
    node: Node<'_>,
    content: &str,
    module: &str,
    kind: SymbolKind,
    facts: &mut JavaScriptSymbolFacts,
) {
    let mut cursor = node.walk();
    let Some(clause) = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == "import_clause")
    else {
        return;
    };
    let mut cursor = clause.walk();
    for binding in clause
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "identifier")
    {
        let Ok(name) = binding.utf8_text(content.as_bytes()) else {
            continue;
        };
        let (line_start, line_end) = span(binding);
        facts.imports.push(ImportedSymbolFact {
            imported_name: "default".to_string(),
            local_name: name.to_string(),
            kind,
            module_specifier: module.to_string(),
            line_start,
            line_end,
            byte_start: binding.start_byte(),
            byte_end: binding.end_byte(),
        });
    }
}

// CommonJS/TS interop supply is outside the ESM proof. Keep it uncertain.
pub(super) fn uncertain_supply(statement: Node<'_>, content: &str) -> bool {
    let Some(assignment) = statement
        .named_child(0)
        .filter(|node| node.kind() == "assignment_expression")
    else {
        return false;
    };
    let Some(mut left) = assignment.child_by_field_name("left") else {
        return false;
    };
    while let Some(object) = left.child_by_field_name("object") {
        left = object;
    }
    left.kind() == "identifier"
        && matches!(
            left.utf8_text(content.as_bytes()).ok(),
            Some("module" | "exports")
        )
}

pub(super) fn has_direct_type_modifier(node: Node<'_>) -> bool {
    super::has_direct_kind(node, "type") || super::has_direct_kind(node, "typeof")
}

pub(super) fn span(node: Node<'_>) -> (usize, usize) {
    (node.start_position().row + 1, node.end_position().row + 1)
}
