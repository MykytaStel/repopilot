mod imports;

use super::LanguageFrontend;
use crate::audits::context::LanguageKind;

use super::ImportExtractor;

static C_FAMILY_IMPORTS: ImportExtractor = ImportExtractor {
    eager: imports::eager,
    deferred: None,
    guarded_optional: None,
    spans: imports::spans,
};

pub(super) static C: LanguageFrontend = frontend("c", "C", LanguageKind::C, &["c"]);
pub(super) static CPP: LanguageFrontend = frontend("cpp", "C++", LanguageKind::Cpp, &["cpp"]);
pub(super) static HEADER: LanguageFrontend = frontend(
    "c-header",
    "C/C++ Header",
    LanguageKind::CHeader,
    &["c-header"],
);

const fn frontend(
    id: &'static str,
    label: &'static str,
    kind: LanguageKind,
    knowledge_ids: &'static [&'static str],
) -> LanguageFrontend {
    LanguageFrontend {
        id,
        label,
        kind,
        knowledge_ids,
        grammars: &[],
        imports: Some(&C_FAMILY_IMPORTS),
        taint: None,
        review: None,
        conventions: &super::conventions::GENERIC_CONVENTIONS,
        risk: None,
        dedicated_risk_audit: None,
        framework_probe: None,
    }
}
