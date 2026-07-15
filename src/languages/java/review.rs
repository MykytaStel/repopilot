//! Review-signal tables for Java: boundary node kinds and algorithmic
//! node-kind sets.

use crate::review::signals::tables::{
    AlgorithmicKinds, BoundaryKinds, RemovedTables, ReviewTables,
};

pub(super) static JAVA_REVIEW: ReviewTables = ReviewTables {
    boundary: Some(&BoundaryKinds {
        decorator_kinds: &["annotation"],
        import_kinds: &["import_declaration"],
    }),
    algorithmic: &AlgorithmicKinds {
        function_kinds: &["method_declaration", "constructor_declaration"],
        loop_kinds: &[
            "for_statement",
            "enhanced_for_statement",
            "while_statement",
            "do_statement",
        ],
        call_kinds: &["method_invocation"],
        control_flow_kinds: &[
            "if_statement",
            "for_statement",
            "enhanced_for_statement",
            "while_statement",
            "do_statement",
            "switch_statement",
            "try_statement",
        ],
        if_kinds: &["if_statement"],
    },
    removed: Some(&JAVA_REMOVED),
};

pub(super) static JAVA_REMOVED: RemovedTables = RemovedTables {
    extensions: &["java"],
    is_test_case: |node, content| {
        (node.kind() == "method_declaration" || node.kind() == "function_declaration")
            && node
                .utf8_text(content.as_bytes())
                .is_ok_and(|text| text.contains("@Test"))
    },
    is_error_handling: |node, _| node.kind() == "try_statement",
    auth_call_kinds: &["method_invocation", "call_expression"],
};
