use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

pub(super) static JAVA: LanguageFrontend = LanguageFrontend {
    id: "java",
    label: "Java",
    kind: LanguageKind::Java,
    knowledge_ids: &["java"],
    grammars: &[GrammarBinding {
        label: "Java",
        grammar: ParseLanguage::Java,
    }],
};
