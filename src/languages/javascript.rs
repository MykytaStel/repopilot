//! The JavaScript dialect family: JavaScript and TypeScript frontends,
//! including their React (`.jsx`/`.tsx`) knowledge-pack dialects.

use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

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
};
