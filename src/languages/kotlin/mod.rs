use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

mod imports;

use super::ImportExtractor;

static KOTLIN_IMPORTS: ImportExtractor = ImportExtractor {
    eager: imports::eager,
    deferred: None,
    spans: imports::spans,
};

pub(super) static KOTLIN: LanguageFrontend = LanguageFrontend {
    id: "kotlin",
    label: "Kotlin",
    kind: LanguageKind::Kotlin,
    knowledge_ids: &["kotlin"],
    grammars: &[GrammarBinding {
        label: "Kotlin",
        grammar: ParseLanguage::Kotlin,
    }],
    imports: Some(&KOTLIN_IMPORTS),
};
