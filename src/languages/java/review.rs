//! Review-signal tables for Java: boundary node kinds and algorithmic
//! node-kind sets.

use crate::review::signals::tables::{AlgorithmicKinds, BoundaryKinds, ReviewTables};

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
};
