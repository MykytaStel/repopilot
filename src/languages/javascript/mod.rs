//! The JavaScript dialect family: JavaScript and TypeScript frontends,
//! including their React (`.jsx`/`.tsx`) knowledge-pack dialects.

use super::conventions::PathConventions;
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
    conventions: &JS_FAMILY_CONVENTIONS,
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
    conventions: &JS_FAMILY_CONVENTIONS,
    risk: Some(&risk::JS_FAMILY_RISK),
};

static JS_FAMILY_CONVENTIONS: PathConventions = PathConventions {
    test_file_name: |name| {
        name.ends_with(".test.ts")
            || name.ends_with(".test.tsx")
            || name.ends_with(".test.js")
            || name.ends_with(".test.jsx")
            || name.ends_with(".spec.ts")
            || name.ends_with(".spec.tsx")
            || name.ends_with(".spec.js")
            || name.ends_with(".spec.jsx")
    },
    test_prefix_marks_test: true,
    test_support: None,
    entrypoint_content: None,
};
