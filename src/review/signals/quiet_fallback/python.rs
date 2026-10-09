use crate::review::diff::ChangedFile;
use crate::review::signals::behavioral::{
    BehavioralKind, BehavioralSignal, BehavioralSignalSource,
};
use crate::review::signals::content::ReviewSource;
use tree_sitter::Node;

pub(super) fn detect(file: &ChangedFile, post: &ReviewSource) -> Vec<BehavioralSignal> {
    let Some(tree) = post.tree().filter(|tree| !tree.root_node().has_error()) else {
        return Vec::new();
    };

    let mut signals = Vec::new();
    visit(tree.root_node(), file, post.content(), &mut signals);
    signals
}

fn visit(node: Node<'_>, file: &ChangedFile, content: &str, signals: &mut Vec<BehavioralSignal>) {
    if node.kind() == "try_statement" {
        signals.extend(quiet_fallbacks(node, file, content));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, file, content, signals);
    }
}

fn quiet_fallbacks(try_node: Node<'_>, file: &ChangedFile, content: &str) -> Vec<BehavioralSignal> {
    let Some(body) = try_node.child_by_field_name("body") else {
        return Vec::new();
    };
    let Some(operation) = first_operation(body, true) else {
        return Vec::new();
    };
    let Some(function) = enclosing_function(try_node) else {
        return Vec::new();
    };
    let Some(return_line) = later_return(function, try_node.end_byte(), true) else {
        return Vec::new();
    };

    except_clauses(try_node)
        .filter_map(|handler| quiet_fallback(handler, operation, return_line, file, content))
        .collect()
}

fn quiet_fallback(
    handler: Node<'_>,
    operation: Node<'_>,
    return_line: usize,
    file: &ChangedFile,
    content: &str,
) -> Option<BehavioralSignal> {
    let line = changed_pass_line(file, handler)?;
    let operation_line = operation.start_position().row + 1;
    let operation_text = operation_source(operation)
        .and_then(|node| node.utf8_text(content.as_bytes()).ok())
        .map(compact_operation)
        .unwrap_or_else(|| "operation".to_string());

    Some(BehavioralSignal {
        kind: BehavioralKind::QuietFallbackIntroduced,
        path: file.path_string(),
        line,
        detail: format!(
            "Operation `{operation_text}` at line {operation_line} can fail inside the changed pass-only exception handler; execution may continue to return line {return_line}. Verify the fallback preserves required behavior."
        ),
        source: BehavioralSignalSource::Ast,
    })
}

fn except_clauses(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| child.kind() == "except_clause")
        .collect::<Vec<_>>()
        .into_iter()
}

fn changed_line(file: &ChangedFile, node: Node<'_>) -> Option<usize> {
    let first = node.start_position().row + 1;
    let last = node.end_position().row + 1;
    (first..=last).find(|line| file.contains_line(*line))
}

fn changed_pass_line(file: &ChangedFile, handler: Node<'_>) -> Option<usize> {
    let body = named_child(handler, "block")?;
    let mut cursor = body.walk();
    let mut changed_pass = None;
    for statement in body.named_children(&mut cursor) {
        if statement.kind() == "comment" {
            continue;
        }
        if statement.kind() != "pass_statement" {
            return None;
        }
        if changed_pass.is_none() {
            changed_pass = changed_line(file, statement);
        }
    }
    changed_pass
}

fn first_operation(node: Node<'_>, is_root: bool) -> Option<Node<'_>> {
    if matches!(
        node.kind(),
        "call" | "import_statement" | "import_from_statement"
    ) {
        return Some(node);
    }
    if node.kind() == "generator_expression" {
        return first_generator_iterable_operation(node);
    }
    if !is_root && is_scope(node.kind()) {
        return None;
    }
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find_map(|child| first_operation(child, false))
}

fn first_generator_iterable_operation(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == "for_in_clause")
        .and_then(|clause| clause.child_by_field_name("right"))
        .and_then(|iterable| first_operation(iterable, false))
}

fn operation_source(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "call" {
        node.child_by_field_name("function")
    } else {
        Some(node)
    }
}

fn later_return(node: Node<'_>, after_byte: usize, is_root: bool) -> Option<usize> {
    if node.kind() == "return_statement" && node.start_byte() > after_byte {
        return Some(node.start_position().row + 1);
    }
    if !is_root && is_scope(node.kind()) {
        return None;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(line) = later_return(child, after_byte, false) {
            return Some(line);
        }
    }
    None
}

fn enclosing_function(node: Node<'_>) -> Option<Node<'_>> {
    let mut parent = node.parent();
    while let Some(candidate) = parent {
        if candidate.kind() == "function_definition" {
            return Some(candidate);
        }
        parent = candidate.parent();
    }
    None
}

fn is_scope(kind: &str) -> bool {
    matches!(
        kind,
        "function_definition" | "class_definition" | "lambda" | "generator_expression"
    )
}

fn named_child<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == kind)
}

fn compact_operation(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(80).collect()
}
