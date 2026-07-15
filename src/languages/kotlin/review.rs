//! Review-signal tables for Kotlin: boundary node kinds and algorithmic
//! node-kind sets.

use crate::review::signals::tables::{
    AlgorithmicKinds, BoundaryKinds, RemovedTables, ReviewTables,
};

pub(super) static KOTLIN_REVIEW: ReviewTables = ReviewTables {
    boundary: Some(&BoundaryKinds {
        decorator_kinds: &["annotation"],
        import_kinds: &["import_declaration"],
    }),
    algorithmic: &AlgorithmicKinds {
        function_kinds: &["function_declaration"],
        loop_kinds: &["for_statement", "while_statement", "do_while_statement"],
        call_kinds: &["call_expression"],
        control_flow_kinds: &[
            "if_expression",
            "when_expression",
            "for_statement",
            "while_statement",
            "do_while_statement",
            "try_expression",
        ],
        if_kinds: &["if_expression"],
    },
    removed: Some(&KOTLIN_REMOVED),
};

pub(super) static KOTLIN_REMOVED: RemovedTables = RemovedTables {
    extensions: &["kt", "kts"],
    is_test_case: |node, content| {
        (node.kind() == "method_declaration" || node.kind() == "function_declaration")
            && node
                .utf8_text(content.as_bytes())
                .is_ok_and(|text| text.contains("@Test"))
    },
    is_error_handling: |node, _| node.kind() == "try_expression",
    auth_call_kinds: &["method_invocation", "call_expression"],
};
