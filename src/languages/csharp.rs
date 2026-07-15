use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;
use crate::review::signals::tables::{AlgorithmicKinds, ReviewTables};

pub(super) static CSHARP: LanguageFrontend = LanguageFrontend {
    id: "csharp",
    label: "C#",
    kind: LanguageKind::CSharp,
    knowledge_ids: &["csharp"],
    // Both labels appear in the wild: detection emits "C#", while some
    // callers pass the enum-style "CSharp" (mirrors ParseLanguage::from_label).
    grammars: &[
        GrammarBinding {
            label: "C#",
            grammar: ParseLanguage::CSharp,
        },
        GrammarBinding {
            label: "CSharp",
            grammar: ParseLanguage::CSharp,
        },
    ],
    imports: None,
    taint: None,
    review: Some(&CSHARP_REVIEW),
};

// Boundary stays unwired: the old dispatch matched the label "CSharp", but
// detection emits "C#", so C# never received AST boundary classification.
// Enabling it is a deliberate behavior change for the honesty pass, not a
// refactor side effect.
static CSHARP_REVIEW: ReviewTables = ReviewTables {
    boundary: None,
    algorithmic: &AlgorithmicKinds {
        function_kinds: &[
            "method_declaration",
            "constructor_declaration",
            "local_function_statement",
        ],
        loop_kinds: &[
            "for_statement",
            "foreach_statement",
            "while_statement",
            "do_statement",
        ],
        call_kinds: &["invocation_expression"],
        control_flow_kinds: &[
            "if_statement",
            "for_statement",
            "foreach_statement",
            "while_statement",
            "do_statement",
            "switch_statement",
            "try_statement",
        ],
        if_kinds: &["if_statement"],
    },
};
