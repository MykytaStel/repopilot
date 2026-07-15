use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

pub(super) static PYTHON: LanguageFrontend = LanguageFrontend {
    id: "python",
    label: "Python",
    kind: LanguageKind::Python,
    knowledge_ids: &["python"],
    grammars: &[GrammarBinding {
        label: "Python",
        grammar: ParseLanguage::Python,
    }],
};
