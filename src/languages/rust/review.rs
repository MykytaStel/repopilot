//! Review-signal tables for Rust: boundary node kinds and algorithmic
//! node-kind sets.

use crate::review::signals::tables::{AlgorithmicKinds, BoundaryKinds, ReviewTables};

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
};
