//! Skip markers for Rust tests: the `#[ignore]` attribute, optionally with a
//! reason (`#[ignore = "slow"]`). Unit tests live inline in `src/`, so the
//! recognizer applies to every Rust file, not only those under `tests/`.

use crate::review::signals::integrity::syntax::{bounded, unquote};
use crate::review::signals::tables::{IntegrityTables, TestMarker, TestMarkerKind};
use tree_sitter::Node;

pub(super) static RUST_INTEGRITY: IntegrityTables = IntegrityTables {
    extensions: &["rs"],
    applies_outside_test_files: true,
    test_marker,
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
