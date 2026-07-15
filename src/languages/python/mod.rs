use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

mod imports;
mod review;

use super::ImportExtractor;

static PYTHON_IMPORTS: ImportExtractor = ImportExtractor {
    eager: imports::eager,
    deferred: Some(imports::deferred),
    spans: imports::spans,
};

pub(super) static PYTHON: LanguageFrontend = LanguageFrontend {
    id: "python",
    label: "Python",
    kind: LanguageKind::Python,
    knowledge_ids: &["python"],
    grammars: &[GrammarBinding {
        label: "Python",
        grammar: ParseLanguage::Python,
    }],
    imports: Some(&PYTHON_IMPORTS),
    taint: Some(&review::PYTHON_TAINT),
    review: Some(&review::PYTHON_REVIEW),
};
