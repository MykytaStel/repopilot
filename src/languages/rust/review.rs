//! Review-signal tables for Rust: boundary node kinds and algorithmic
//! node-kind sets.

use crate::review::signals::behavioral::removed_ast::callee_text;
use crate::review::signals::tables::{
    AlgorithmicKinds, BoundaryKinds, RemovedTables, ReviewTables,
};

pub(super) static RUST_REVIEW: ReviewTables = ReviewTables {
    boundary: Some(&BoundaryKinds {
        decorator_kinds: &["attribute_item", "use_declaration"],
        import_kinds: &[],
    }),
    algorithmic: &AlgorithmicKinds {
        function_kinds: &["function_item"],
        loop_kinds: &["for_expression", "while_expression", "loop_expression"],
        call_kinds: &["call_expression"],
        control_flow_kinds: &[
            "if_expression",
            "for_expression",
            "while_expression",
            "loop_expression",
            "match_expression",
        ],
        if_kinds: &["if_expression"],
    },
    removed: Some(&RUST_REMOVED),
};

pub(super) static RUST_REMOVED: RemovedTables = RemovedTables {
    extensions: &["rs"],
    is_test_case: |node, content| {
        node.kind() == "function_item"
            && node
                .utf8_text(content.as_bytes())
                .is_ok_and(|text| text.contains("#[test]"))
    },
    is_error_handling: |node, content| {
        let kind = node.kind();
        let text = node.utf8_text(content.as_bytes()).unwrap_or("");
        (kind == "match_expression" && text.contains("Err("))
            || (kind == "if_let_expression" && text.contains("Err("))
            // Match on the callee (`x.map_err`), not the whole call, so a
            // `.unwrap_or` buried in an argument does not double-count.
            || (kind == "call_expression"
                && callee_text(node, content).is_some_and(|callee: &str| {
                    callee.contains(".unwrap_or") || callee.contains(".map_err")
                }))
    },
    auth_call_kinds: &["call_expression"],
};
