use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

mod imports;
mod review;

use super::ImportExtractor;

static GO_IMPORTS: ImportExtractor = ImportExtractor {
    eager: imports::eager,
    deferred: None,
    spans: imports::spans,
};

pub(super) static GO: LanguageFrontend = LanguageFrontend {
    id: "go",
    label: "Go",
    kind: LanguageKind::Go,
    knowledge_ids: &["go"],
    grammars: &[GrammarBinding {
        label: "Go",
        grammar: ParseLanguage::Go,
    }],
    imports: Some(&GO_IMPORTS),
    taint: Some(&review::GO_TAINT),
    review: Some(&review::GO_REVIEW),
};
