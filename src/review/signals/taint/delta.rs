//! Bounded before/after analysis for an unchanged sink immediately following
//! a changed local declaration. This is not general control-flow analysis.

use super::{TaintSignal, detect_taint};
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
use crate::review::signals::content::ReviewSource;
use tree_sitter::Node;

/// Preserve the existing changed-sink detector. Add evidence only when the
/// same supported sink is clean before and tainted after a single-line edit.
pub(super) fn detect_unchanged_sink(
    file: &ChangedFile,
    pre: Option<&ReviewSource>,
    post: Option<&ReviewSource>,
) -> Vec<TaintSignal> {
    if file.status != ChangeStatus::Modified
        || !matches!(
            file.path.extension().and_then(|ext| ext.to_str()),
            Some("js" | "jsx" | "ts" | "tsx")
        )
        || crate::audits::context::classify::helpers::is_test_file(&file.path)
    {
        return Vec::new();
    }
    let (Some(pre), Some(post)) = (pre, post) else {
        return Vec::new();
    };
    let [range] = file.ranges.as_slice() else {
        return Vec::new();
    };
    if range.start != range.end || range.start == 0 {
        return Vec::new();
    }
    let row = range.start - 1;
    if !single_line_replacement(pre.content(), post.content(), row) {
        return Vec::new();
    }
    let (Some(before), Some(after)) = (pre.tree(), post.tree()) else {
        return Vec::new();
    };
    if before.root_node().has_error() || after.root_node().has_error() {
        return Vec::new();
    }
    let (Some(old), Some(new)) = (
        adjacent_site(before.root_node(), pre.content(), row),
        adjacent_site(after.root_node(), post.content(), row),
    ) else {
        return Vec::new();
    };
    if old != new || file.contains_line(new.sink_line) {
        return Vec::new();
    }

    // Ask the existing detector about the same unchanged sink on both sides.
    // Do not widen the original diff or change the public changed-line API.
    let probe = ChangedFile {
        path: file.path.clone(),
        status: file.status,
        ranges: vec![ChangedRange {
            start: new.sink_line,
            end: new.sink_line,
        }],
        hunks: Vec::new(),
    };
    let previous = detect_taint(&probe, Some(pre));
    detect_taint(&probe, Some(post))
        .into_iter()
        .filter(|signal| {
            signal.line == new.sink_line
                && !previous.iter().any(|old| {
                    old.line == signal.line && old.sink == signal.sink
                })
        })
        .map(|mut signal| {
            signal.detail = format!(
                "New flow after declaration change at line {}: {}",
                range.start, signal.detail
            );
            signal
        })
        .collect()
}

fn single_line_replacement(before: &str, after: &str, row: usize) -> bool {
    let mut before = before.lines();
    let mut after = after.lines();
    let mut index = 0;
    let mut changed = false;
    loop {
        match (before.next(), after.next()) {
            (Some(a), Some(b)) => {
                if a != b {
                    if index != row {
                        return false;
                    }
                    changed = true;
                }
                index += 1;
            }
            (None, None) => return changed,
            _ => return false,
        }
    }
}

#[derive(PartialEq, Eq)]
struct Site<'a> {
    name: &'a str,
    sink_line: usize,
    sink_text: &'a str,
}

fn adjacent_site<'a>(node: Node<'_>, content: &'a str, row: usize) -> Option<Site<'a>> {
    if node.start_position().row > row || node.end_position().row < row {
        return None;
    }
    if matches!(node.kind(), "lexical_declaration" | "variable_declaration") {
        return declaration_site(node, content, row);
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find_map(|child| adjacent_site(child, content, row))
}

fn declaration_site<'a>(
    node: Node<'_>,
    content: &'a str,
    row: usize,
) -> Option<Site<'a>> {
    if node.start_position().row != row
        || node.end_position().row != row
        || !matches!(node.parent()?.kind(), "program" | "statement_block")
    {
        return None;
    }
    let mut cursor = node.walk();
    let mut children = node
        .named_children(&mut cursor)
        .filter(|child| child.kind() != "comment");
    let declaration = children.next()?;
    if declaration.kind() != "variable_declarator" || children.next().is_some() {
        return None;
    }
    let name = declaration.child_by_field_name("name")?;
    if name.kind() != "identifier" {
        return None;
    }
    declaration.child_by_field_name("value")?;
    let mut next = node.next_named_sibling()?;
    while next.kind() == "comment" {
        next = next.next_named_sibling()?;
    }
    if next.kind() != "expression_statement"
        || next.start_position().row <= row
        || next.start_position().row != next.end_position().row
    {
        return None;
    }
    let call = next.named_child(0)?;
    if call.kind() != "call_expression" {
        return None;
    }
    let argument = call.child_by_field_name("arguments")?.named_child(0)?;
    // Limit the pilot to a direct identifier argument; no alias, callback,
    // conditional, or helper expansion is needed to identify this local use.
    if argument.kind() != "identifier"
        || argument.utf8_text(content.as_bytes()).ok()? != name.utf8_text(content.as_bytes()).ok()?
    {
        return None;
    }
    Some(Site {
        name: name.utf8_text(content.as_bytes()).ok()?,
        sink_line: call.start_position().row + 1,
        sink_text: call.utf8_text(content.as_bytes()).ok()?,
    })
}
