//! Test-integrity recognizers for Rust: the `#[ignore]` attribute (optionally
//! with a reason), `#[test]`-attributed cases, and `assert!`-family macros.
//! Unit tests live inline in `src/`, so the recognizers apply to every Rust
//! file, not only those under `tests/`.

use crate::review::signals::integrity::syntax::{bounded, unquote};
use crate::review::signals::tables::{IntegrityTables, TestMarker, TestMarkerKind};
use tree_sitter::Node;

pub(super) static RUST_INTEGRITY: IntegrityTables = IntegrityTables {
    extensions: &["rs"],
    applies_outside_test_files: true,
    test_marker,
    test_case,
    is_assertion,
};

fn test_marker<'a>(node: Node<'a>, content: &'a str) -> Option<TestMarker> {
    if node.kind() != "attribute_item" {
        return None;
    }
    let text = node.utf8_text(content.as_bytes()).ok()?;
    let inner = text.trim().strip_prefix("#[")?.strip_suffix(']')?.trim();
    let path: String = inner
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
        .collect();
    if path != "ignore" {
        return None;
    }
    let reason = inner[path.len()..]
        .trim()
        .strip_prefix('=')
        .map(|value| bounded(unquote(value)));
    Some(TestMarker {
        kind: TestMarkerKind::Skip,
        marker: "#[ignore]".to_string(),
        test_name: annotated_function(node, content),
        reason,
        line: node.start_position().row + 1,
    })
}

/// The function an attribute applies to: the next sibling item after any
/// further attributes or comments.
fn annotated_function(node: Node<'_>, content: &str) -> Option<String> {
    let mut sibling = node.next_named_sibling();
    while let Some(item) = sibling {
        match item.kind() {
            "attribute_item" | "line_comment" | "block_comment" => {
                sibling = item.next_named_sibling();
            }
            "function_item" => {
                return item
                    .child_by_field_name("name")
                    .and_then(|name| name.utf8_text(content.as_bytes()).ok())
                    .map(bounded);
            }
            _ => return None,
        }
    }
    None
}

fn test_case<'a>(node: Node<'a>, content: &'a str) -> Option<String> {
    if node.kind() != "function_item" || !has_test_attribute(node, content) {
        return None;
    }
    let name = node
        .child_by_field_name("name")?
        .utf8_text(content.as_bytes())
        .ok()?;
    let mut qualified = vec![name.to_string()];
    let mut current = node.parent();
    while let Some(candidate) = current {
        if candidate.kind() == "mod_item"
            && let Some(module) = candidate
                .child_by_field_name("name")
                .and_then(|module| module.utf8_text(content.as_bytes()).ok())
        {
            qualified.push(module.to_string());
        }
        current = candidate.parent();
    }
    qualified.reverse();
    Some(qualified.join("::"))
}

/// Attributes are siblings that precede the item in tree-sitter-rust.
pub(super) fn has_test_attribute(node: Node<'_>, content: &str) -> bool {
    let mut sibling = node.prev_named_sibling();
    while let Some(item) = sibling {
        match item.kind() {
            "attribute_item" => {
                let path: String = item
                    .utf8_text(content.as_bytes())
                    .unwrap_or_default()
                    .trim_start_matches("#[")
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
                    .collect();
                if path == "test" || path.ends_with("::test") || path == "rstest" {
                    return true;
                }
                sibling = item.prev_named_sibling();
            }
            "line_comment" | "block_comment" => sibling = item.prev_named_sibling(),
            _ => return false,
        }
    }
    false
}

const ASSERT_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "assert_matches",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "prop_assert",
    "prop_assert_eq",
    "prop_assert_ne",
];

fn is_assertion<'a>(node: Node<'a>, content: &'a str) -> bool {
    if node.kind() != "macro_invocation" {
        return false;
    }
    let Some(name) = node
        .child_by_field_name("macro")
        .and_then(|name| name.utf8_text(content.as_bytes()).ok())
    else {
        return false;
    };
    if !ASSERT_MACROS.contains(&name) {
        return false;
    }
    // `assert!(true)` and `assert_eq!(1, 1)` cannot fail.
    let arguments: String = node
        .utf8_text(content.as_bytes())
        .unwrap_or_default()
        .split_once('!')
        .map(|(_, rest)| rest.split_whitespace().collect())
        .unwrap_or_default();
    let inner = arguments.trim_start_matches('(').trim_end_matches(')');
    let parts: Vec<&str> = inner.split(',').filter(|part| !part.is_empty()).collect();
    let constant = |part: &str| {
        part == "true"
            || part == "false"
            || part.parse::<f64>().is_ok()
            || (part.starts_with('"') && part.ends_with('"'))
    };
    !(parts.len() <= 2 && !parts.is_empty() && parts.iter().all(|part| constant(part)))
}
