//! Computed frontend capabilities.
//!
//! A capability is never declared — it is derived from what a frontend has
//! actually wired. The knowledge pack *declares* a support level per
//! language; the registry *computes* one from capabilities. The guard tests
//! keep a shrinking ledger of languages whose declared level exceeds the
//! computed one, so the support matrix cannot silently over-promise.

use super::LanguageFrontend;
use crate::knowledge::model::SupportLevel;

/// What a frontend can actually do, judged by its wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Has a bundled tree-sitter grammar.
    Parse,
    /// Owns an import extractor (arrives with the imports migration).
    Imports,
    /// Owns review signal tables — boundary, behavioral, algorithmic.
    ReviewSignals,
    /// Owns taint source/sink/sanitizer tables.
    TaintFlows,
    /// Owns runtime-risk patterns.
    RuntimeRisk,
    /// Owns test-file and test-support path conventions.
    TestConventions,
    /// Owns framework probes.
    Frameworks,
}

/// Capabilities currently wired for `frontend`. Only [`Capability::Parse`]
/// is derivable in the skeleton; each migration PR extends this from the
/// fields it adds to [`LanguageFrontend`].
pub fn capabilities(frontend: &LanguageFrontend) -> Vec<Capability> {
    let mut wired = Vec::new();
    if !frontend.grammars.is_empty() {
        wired.push(Capability::Parse);
    }
    wired
}

/// The support level the current wiring justifies.
///
/// `rule-aware` requires the full set a rule-aware language actually uses;
/// `context-aware` requires at least a grammar; anything with an import
/// extractor but no grammar is `import-aware`; the rest is `detect-only`.
pub fn computed_support(frontend: &LanguageFrontend) -> SupportLevel {
    let wired = capabilities(frontend);
    let has = |capability: Capability| wired.contains(&capability);

    if has(Capability::Parse)
        && has(Capability::Imports)
        && has(Capability::ReviewSignals)
        && has(Capability::RuntimeRisk)
        && has(Capability::TestConventions)
    {
        SupportLevel::RuleAware
    } else if has(Capability::Parse) {
        SupportLevel::ContextAware
    } else if has(Capability::Imports) {
        SupportLevel::ImportAware
    } else {
        SupportLevel::DetectOnly
    }
}

/// Total order for comparing declared vs computed support in guard tests.
pub fn support_rank(level: SupportLevel) -> u8 {
    match level {
        SupportLevel::DetectOnly => 0,
        SupportLevel::ImportAware => 1,
        SupportLevel::ContextAware => 2,
        SupportLevel::RuleAware => 3,
    }
}
