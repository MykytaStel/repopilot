use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

mod imports;

use super::ImportExtractor;

static JAVA_IMPORTS: ImportExtractor = ImportExtractor {
    eager: imports::eager,
    deferred: None,
    spans: imports::spans,
};

pub(super) static JAVA: LanguageFrontend = LanguageFrontend {
    id: "java",
    label: "Java",
    kind: LanguageKind::Java,
    knowledge_ids: &["java"],
    grammars: &[GrammarBinding {
        label: "Java",
        grammar: ParseLanguage::Java,
    }],
    imports: Some(&JAVA_IMPORTS),
};
