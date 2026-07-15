use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

mod imports;

use super::ImportExtractor;

static RUST_IMPORTS: ImportExtractor = ImportExtractor {
    eager: imports::eager,
    deferred: None,
    spans: imports::spans,
};

pub(super) static RUST: LanguageFrontend = LanguageFrontend {
    id: "rust",
    label: "Rust",
    kind: LanguageKind::Rust,
    knowledge_ids: &["rust"],
    grammars: &[GrammarBinding {
        label: "Rust",
        grammar: ParseLanguage::Rust,
    }],
    imports: Some(&RUST_IMPORTS),
};
