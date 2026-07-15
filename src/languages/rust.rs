use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

pub(super) static RUST: LanguageFrontend = LanguageFrontend {
    id: "rust",
    label: "Rust",
    kind: LanguageKind::Rust,
    knowledge_ids: &["rust"],
    grammars: &[GrammarBinding {
        label: "Rust",
        grammar: ParseLanguage::Rust,
    }],
};
