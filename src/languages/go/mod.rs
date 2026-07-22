use super::conventions::PathConventions;
use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

mod imports;
mod review;
mod risk;

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
    conventions: &GO_CONVENTIONS,
    risk: Some(&risk::GO_RISK),
    dedicated_risk_audit: None,
};

static GO_CONVENTIONS: PathConventions = PathConventions {
    test_file_name: |name| name.ends_with("_test.go"),
    test_prefix_marks_test: true,
    test_support: None,
    entrypoint_content: Some(|content| content.contains("func main(")),
};
