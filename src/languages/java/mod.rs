use super::conventions::{MANAGED_TEST_SUPPORT, PathConventions};
use super::{GrammarBinding, LanguageFrontend};
use crate::analysis::parse::ParseLanguage;
use crate::audits::context::LanguageKind;

mod imports;
mod review;
mod risk;

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
    taint: Some(&review::JAVA_TAINT),
    review: Some(&review::JAVA_REVIEW),
    conventions: &JAVA_CONVENTIONS,
    risk: Some(&risk::JAVA_RISK),
    dedicated_risk_audit: None,
};

static JAVA_CONVENTIONS: PathConventions = PathConventions {
    test_file_name: |name| name.ends_with("test.java") || name.ends_with("tests.java"),
    test_prefix_marks_test: true,
    test_support: Some(&MANAGED_TEST_SUPPORT),
    entrypoint_content: None,
};
