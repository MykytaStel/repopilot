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
    if node.kind() == "if_statement"
        && let Some(signal) = quiet_fallback(node, file, content)
    {
        signals.push(signal);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, file, content, signals);
    }
}

fn quiet_fallback(
    if_node: Node<'_>,
    file: &ChangedFile,
    content: &str,
) -> Option<BehavioralSignal> {
    if if_node.child_by_field_name("alternative").is_some() {
        return None;
    }
    let condition = if_node.child_by_field_name("condition")?;
    let err_ident = error_not_nil_identifier(condition, content)?;
    let consequence = if_node.child_by_field_name("consequence")?;
    if !is_empty_block(consequence) {
        return None;
    }
    let line = changed_line(file, if_node)?;
    let function = enclosing_function(if_node)?;
    let return_line = later_return(function, if_node.end_byte(), true)?;
    let (call_line, operation) = preceding_call_for_err(if_node, function, &err_ident, content)
        .unwrap_or((if_node.start_position().row + 1, "operation".to_string()));

    Some(BehavioralSignal {
        kind: BehavioralKind::QuietFallbackIntroduced,
        path: file.path_string(),
        line,
        detail: format!(
            "Call `{operation}` at line {call_line} can fail inside the changed empty `if {err_ident} != nil` block; execution may continue to return line {return_line}. Verify the fallback preserves required behavior."
        ),
        source: BehavioralSignalSource::Ast,
    })
}

fn error_not_nil_identifier(condition: Node<'_>, content: &str) -> Option<String> {
    if condition.kind() != "binary_expression" {
        return None;
    }
    let text = condition.utf8_text(content.as_bytes()).ok()?;
    let compact: String = text.split_whitespace().collect();
    let ident = compact
        .strip_suffix("!=nil")
        .or_else(|| compact.strip_prefix("nil!="))?;
    let lower = ident.to_ascii_lowercase();
    (lower == "err" || lower.ends_with("err")).then(|| ident.to_string())
}

fn is_empty_block(block: Node<'_>) -> bool {
    let mut cursor = block.walk();
    block
        .named_children(&mut cursor)
        .all(|child| matches!(child.kind(), "comment" | "empty_statement"))
}

fn changed_line(file: &ChangedFile, node: Node<'_>) -> Option<usize> {
    let first = node.start_position().row + 1;
    let last = node.end_position().row + 1;
    (first..=last).find(|line| file.contains_line(*line))
}

fn preceding_call_for_err(
    if_node: Node<'_>,
    function: Node<'_>,
    err_ident: &str,
    content: &str,
) -> Option<(usize, String)> {
    if let Some(init) = if_node.child_by_field_name("initializer")
        && let Some(call) = first_call(init)
    {
        return call_summary(call, content);
    }
    let mut prev = if_node.prev_named_sibling();
    while let Some(stmt) = prev {
        if stmt.kind() == "comment" {
            prev = stmt.prev_named_sibling();
            continue;
        }
        if let Ok(text) = stmt.utf8_text(content.as_bytes())
            && text.contains(err_ident)
            && let Some(call) = first_call(stmt)
        {
            return call_summary(call, content);
        }
        break;
    }
    first_call(function).and_then(|call| call_summary(call, content))
}

fn first_call(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "call_expression" {
        return Some(node);
    }
    let mut cursor = node.walk();
    node.children(&mut cursor).find_map(first_call)
}

fn call_summary(call: Node<'_>, content: &str) -> Option<(usize, String)> {
    let line = call.start_position().row + 1;
    let callee = call
        .child_by_field_name("function")
        .unwrap_or(call)
        .utf8_text(content.as_bytes())
        .ok()?;
    let compact = callee.split_whitespace().collect::<Vec<_>>().join(" ");
    Some((line, compact.chars().take(80).collect()))
}

fn enclosing_function(node: Node<'_>) -> Option<Node<'_>> {
    let mut parent = node.parent();
    while let Some(candidate) = parent {
        if is_function_scope(candidate.kind()) {
            return Some(candidate);
        }
        parent = candidate.parent();
    }
    None
}

fn later_return(node: Node<'_>, after_byte: usize, is_root: bool) -> Option<usize> {
    if node.kind() == "return_statement" && node.start_byte() > after_byte {
        return Some(node.start_position().row + 1);
    }
    if !is_root && is_function_scope(node.kind()) {
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

fn is_function_scope(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration" | "method_declaration" | "func_literal"
    )
}

#[cfg(test)]
mod tests {
    use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
    use crate::review::signals::behavioral::{
        BehavioralKind, DependencyContext, detect_behavioral_added,
    };
    use crate::review::signals::content::ReviewSource;
    use std::path::PathBuf;

    fn run(source: &str, changed_line: usize) -> Vec<String> {
        let file = ChangedFile {
            path: PathBuf::from("pkg/loader.go"),
            status: ChangeStatus::Modified,
            ranges: vec![ChangedRange {
                start: changed_line,
                end: changed_line,
            }],
            hunks: Vec::new(),
        };
        let src = ReviewSource::new(source.to_string(), Some("Go".to_string()));
        detect_behavioral_added(&file, &src, &DependencyContext::default())
            .into_iter()
            .filter(|s| s.kind == BehavioralKind::QuietFallbackIntroduced)
            .map(|s| s.detail)
            .collect()
    }

    #[test]
    fn empty_if_err_not_nil_with_later_return_is_reported() {
        let go = "package pkg\nfunc Load() string {\n\tv, err := fetch()\n\tif err != nil {\n\t\t// ignore\n\t}\n\treturn v\n}\n";
        let details = run(go, 4);
        assert_eq!(details.len(), 1, "{details:?}");
        assert!(details[0].contains("fetch"), "{}", details[0]);
    }

    #[test]
    fn non_empty_if_err_or_missing_later_return_is_not_reported() {
        let handled = "package pkg\nfunc Load() error {\n\t_, err := fetch()\n\tif err != nil {\n\t\treturn err\n\t}\n\treturn nil\n}\n";
        let no_return =
            "package pkg\nfunc Log() {\n\t_, err := fetch()\n\tif err != nil {\n\t}\n}\n";
        assert!(run(handled, 4).is_empty());
        assert!(run(no_return, 4).is_empty());
    }
}
