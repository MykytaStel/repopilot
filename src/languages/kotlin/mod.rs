use super::conventions::{MANAGED_TEST_SUPPORT, PathConventions};
use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

mod imports;
mod review;
mod risk;

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
    taint: Some(&review::KOTLIN_TAINT),
    review: Some(&review::KOTLIN_REVIEW),
    conventions: &KOTLIN_CONVENTIONS,
    risk: Some(&risk::KOTLIN_RISK),
    dedicated_risk_audit: None,
    framework_probe: None,
};

static KOTLIN_CONVENTIONS: PathConventions = PathConventions {
    test_file_name: |name| name.ends_with("test.kt") || name.ends_with("tests.kt"),
    test_prefix_marks_test: true,
    test_support: Some(&MANAGED_TEST_SUPPORT),
    entrypoint_content: None,
};
