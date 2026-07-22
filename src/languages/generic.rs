use super::LanguageFrontend;
use crate::audits::context::LanguageKind;

/// Fallback frontend for every language without dedicated wiring. Such
/// files still contribute detection, size, scope, and generic findings —
/// they just have no language-specific extractors.
pub(super) static GENERIC: LanguageFrontend = LanguageFrontend {
    id: "generic",
    label: "Generic",
    kind: LanguageKind::Unknown,
    knowledge_ids: &[],
    grammars: &[],
    imports: None,
    taint: None,
    review: None,
    conventions: &super::conventions::GENERIC_CONVENTIONS,
    risk: None,
    dedicated_risk_audit: None,
};
