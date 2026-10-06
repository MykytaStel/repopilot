use super::super::{ExportedSymbolFact, JavaScriptSymbolFacts, RustFunctionArityFact, SymbolKind};
use super::{Node, text};

pub(super) fn record_public_function(
    node: Node<'_>,
    content: &str,
    facts: &mut JavaScriptSymbolFacts,
) -> Option<()> {
    let name = node.child_by_field_name("name")?;
    let name: String = text(name, content).into();
    facts.exports.push(ExportedSymbolFact {
        name: name.clone(),
        kind: SymbolKind::Value,
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
    });
    if let Some(parameter_count) = function_arity(node) {
        facts
            .rust_contracts
            .as_mut()?
            .functions
            .push(RustFunctionArityFact {
                name,
                parameter_count,
            });
    }
    Some(())
}

fn function_arity(node: Node<'_>) -> Option<usize> {
    if node.child_by_field_name("type_parameters").is_some() {
        return None;
    }
    let parameters = node.child_by_field_name("parameters")?;
    let mut cursor = parameters.walk();
    let children = parameters.named_children(&mut cursor).collect::<Vec<_>>();
    if children
        .iter()
        .any(|child| child.kind() == "variadic_parameter")
    {
        return None;
    }
    Some(children.len())
}
