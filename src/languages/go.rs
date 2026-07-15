use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

pub(super) static GO: LanguageFrontend = LanguageFrontend {
    id: "go",
    label: "Go",
    kind: LanguageKind::Go,
    knowledge_ids: &["go"],
    grammars: &[GrammarBinding {
        label: "Go",
        grammar: ParseLanguage::Go,
    }],
};
