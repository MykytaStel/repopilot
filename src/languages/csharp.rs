use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

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
};
