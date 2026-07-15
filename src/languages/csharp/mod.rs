mod risk;

use super::conventions::{MANAGED_TEST_SUPPORT, PathConventions};
use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;
use crate::review::signals::tables::{AlgorithmicKinds, RemovedTables, ReviewTables};

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
    conventions: &CSHARP_CONVENTIONS,
    risk: Some(&risk::CSHARP_RISK),
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
    removed: Some(&CSHARP_REMOVED),
};

static CSHARP_REMOVED: RemovedTables = RemovedTables {
    extensions: &["cs"],
    is_test_case: |node, content| {
        node.kind() == "method_declaration"
            && node.utf8_text(content.as_bytes()).is_ok_and(|text| {
                text.contains("[Test]") || text.contains("[TestMethod]") || text.contains("[Fact]")
            })
    },
    is_error_handling: |node, _| node.kind() == "try_statement",
    auth_call_kinds: &["invocation_expression"],
};

static CSHARP_CONVENTIONS: PathConventions = PathConventions {
    test_file_name: |name| name.ends_with("test.cs") || name.ends_with("tests.cs"),
    test_prefix_marks_test: true,
    test_support: Some(&MANAGED_TEST_SUPPORT),
    entrypoint_content: None,
};
