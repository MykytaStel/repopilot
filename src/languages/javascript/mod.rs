//! The JavaScript dialect family: JavaScript and TypeScript frontends,
//! including their React (`.jsx`/`.tsx`) knowledge-pack dialects.

use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

mod imports;
mod review;
mod risk;

use super::ImportExtractor;

static JS_FAMILY_IMPORTS: ImportExtractor = ImportExtractor {
    eager: imports::eager,
    deferred: Some(imports::deferred),
    spans: imports::spans,
};

pub(super) static TYPESCRIPT: LanguageFrontend = LanguageFrontend {
    id: "typescript",
    label: "TypeScript",
    kind: LanguageKind::TypeScript,
    knowledge_ids: &["typescript", "typescript-react"],
    grammars: &[
        GrammarBinding {
            label: "TypeScript",
            grammar: ParseLanguage::TypeScript,
        },
        GrammarBinding {
            label: "TypeScript React",
            grammar: ParseLanguage::Tsx,
        },
    ],
    imports: Some(&JS_FAMILY_IMPORTS),
    taint: Some(&review::JS_FAMILY_TAINT),
    review: Some(&review::JS_FAMILY_REVIEW),
    risk: Some(&risk::JS_FAMILY_RISK),
};

pub(super) static JAVASCRIPT: LanguageFrontend = LanguageFrontend {
    id: "javascript",
    label: "JavaScript",
    kind: LanguageKind::JavaScript,
    knowledge_ids: &["javascript", "javascript-react"],
    grammars: &[
        GrammarBinding {
            label: "JavaScript",
            grammar: ParseLanguage::JavaScript,
        },
        GrammarBinding {
            label: "JavaScript React",
            grammar: ParseLanguage::JavaScript,
        },
    ],
    imports: Some(&JS_FAMILY_IMPORTS),
    taint: Some(&review::JS_FAMILY_TAINT),
    review: Some(&review::JS_FAMILY_REVIEW),
    risk: Some(&risk::JS_FAMILY_RISK),
};
