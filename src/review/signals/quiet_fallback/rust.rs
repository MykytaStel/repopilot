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
    if node.kind() == "match_expression" {
        signals.extend(quiet_match_fallbacks(node, file, content));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, file, content, signals);
    }
}

fn quiet_match_fallbacks(
    match_node: Node<'_>,
    file: &ChangedFile,
    content: &str,
) -> Vec<BehavioralSignal> {
    let Some(scrutinee) = match_node.child_by_field_name("value") else {
        return Vec::new();
    };
    let Some(call) = first_call(scrutinee) else {
        return Vec::new();
    };
    let Some(function) = enclosing_function(match_node) else {
        return Vec::new();
    };
    let Some(return_line) = later_continuation_line(function, match_node.end_byte(), true) else {
        return Vec::new();
    };
    let Some(body) = match_node.child_by_field_name("body") else {
        return Vec::new();
    };

    let call_line = call.start_position().row + 1;
    let operation = call
        .child_by_field_name("function")
        .unwrap_or(call)
        .utf8_text(content.as_bytes())
        .ok()
        .map(compact_operation)
        .unwrap_or_else(|| "operation".to_string());

    let mut cursor = body.walk();
    body.named_children(&mut cursor)
        .filter(|child| child.kind() == "match_arm")
        .filter_map(|arm| {
            arm_quiet_fallback(arm, file, content, &operation, call_line, return_line)
        })
        .collect()
}

fn arm_quiet_fallback(
    arm: Node<'_>,
    file: &ChangedFile,
    content: &str,
    operation: &str,
    call_line: usize,
    return_line: usize,
) -> Option<BehavioralSignal> {
    let pattern = arm.child_by_field_name("pattern")?;
    let pattern_text = pattern.utf8_text(content.as_bytes()).ok()?.trim();
    if !is_err_pattern(pattern_text) {
        return None;
    }
    let value = arm.child_by_field_name("value")?;
    if !is_empty_arm_value(value) {
        return None;
    }
    let line = changed_line(file, arm)?;
    Some(BehavioralSignal {
        kind: BehavioralKind::QuietFallbackIntroduced,
        path: file.path_string(),
        line,
        detail: format!(
            "Call `{operation}` at line {call_line} can fail inside the changed empty `Err` match arm; execution may continue to return line {return_line}. Verify the fallback preserves required behavior."
        ),
        source: BehavioralSignalSource::Ast,
    })
}

fn is_err_pattern(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with("Err(") || trimmed.starts_with("Result::Err(")
}

fn is_empty_arm_value(node: Node<'_>) -> bool {
    match node.kind() {
        "unit_expression" => true,
        "block" => {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .all(|child| matches!(child.kind(), "line_comment" | "block_comment"))
        }
        _ => false,
    }
}

fn changed_line(file: &ChangedFile, node: Node<'_>) -> Option<usize> {
    let first = node.start_position().row + 1;
    let last = node.end_position().row + 1;
    (first..=last).find(|line| file.contains_line(*line))
}

fn first_call(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "call_expression" {
        return Some(node);
    }
    let mut cursor = node.walk();
    node.children(&mut cursor).find_map(first_call)
}

fn compact_operation(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(80).collect()
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

fn later_continuation_line(node: Node<'_>, after_byte: usize, is_root: bool) -> Option<usize> {
    if node.kind() == "return_expression" && node.start_byte() > after_byte {
        return Some(node.start_position().row + 1);
    }
    if !is_root && is_function_scope(node.kind()) {
        return None;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(line) = later_continuation_line(child, after_byte, false) {
            return Some(line);
        }
    }
    if is_root && let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        if let Some(last) = body
            .named_children(&mut cursor)
            .filter(|child| !matches!(child.kind(), "line_comment" | "block_comment"))
            .last()
            && last.start_byte() > after_byte
        {
            return Some(last.start_position().row + 1);
        }
    }
    None
}

fn is_function_scope(kind: &str) -> bool {
    matches!(kind, "function_item" | "closure_expression")
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
            path: PathBuf::from("src/loader.rs"),
            status: ChangeStatus::Modified,
            ranges: vec![ChangedRange {
                start: changed_line,
                end: changed_line,
            }],
            hunks: Vec::new(),
        };
        let src = ReviewSource::new(source.to_string(), Some("Rust".to_string()));
        detect_behavioral_added(&file, &src, &DependencyContext::default())
            .into_iter()
            .filter(|s| s.kind == BehavioralKind::QuietFallbackIntroduced)
            .map(|s| s.detail)
            .collect()
    }

    #[test]
    fn empty_err_match_arm_with_later_return_is_reported() {
        let rust = "pub fn load() -> i32 {\n    match fetch() {\n        Ok(v) => return v,\n        Err(_) => {}\n    }\n    0\n}\n";
        let details = run(rust, 4);
        assert_eq!(details.len(), 1, "{details:?}");
        assert!(details[0].contains("fetch"), "{}", details[0]);
    }

    #[test]
    fn non_empty_err_match_arm_or_no_later_return_is_not_reported() {
        let handled = "pub fn load() -> i32 {\n    match fetch() {\n        Ok(v) => return v,\n        Err(_) => return -1,\n    }\n    0\n}\n";
        let tail_match = "pub fn log() {\n    match fetch() {\n        Ok(_) => {}\n        Err(_) => {}\n    }\n}\n";
        assert!(run(handled, 4).is_empty());
        assert!(run(tail_match, 4).is_empty());
    }
}
