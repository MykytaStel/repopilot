//! Language frontend registry — the single place that answers "what does
//! RepoPilot know about language X".
//!
//! Today language knowledge is spread across detection profiles
//! (`knowledge/packs/core.toml`), grammar wiring (`analysis/parse.rs`),
//! import extractors (`graph/imports/*`), review signal tables, runtime-risk
//! patterns, and path conventions. Each frontend in this registry will own
//! its language's slice of those concerns; consumers look the frontend up by
//! [`LanguageKind`] or knowledge-pack id instead of matching on label
//! strings.
//!
//! This module is the 0.21 skeleton (PR-1): descriptors and guard tests
//! only, wired to nothing. Follow-up PRs migrate dispatch here one concern
//! at a time (detection/parse, imports, review tables, risk, conventions),
//! each behavior-frozen against the zoo snapshots. Per-language files grow
//! into directories when they gain their first extractor.
//!
//! Frontends are static data assembled at compile time — this is not a
//! plugin system, and per the Knowledge Engine runtime boundaries it must
//! not become one.

pub mod capability;
pub(crate) mod conventions;
pub(crate) mod import_support;
mod reference;

mod csharp;
mod generic;
mod go;
mod java;
mod javascript;
mod kotlin;
mod python;
mod rust;

#[cfg(test)]
mod tests;

pub use reference::render_language_support_markdown;

use crate::analysis::parse::{ParseLanguage, ParsedFile};
use crate::audits::context::LanguageKind;
use std::collections::{BTreeMap, HashSet};

/// Binds one detection label (as stored on `FileFacts.language`) to the
/// tree-sitter grammar that parses it. Mirrors `ParseLanguage::from_label`
/// until parse dispatch is rewired through the registry; the guard tests
/// keep the two in lockstep in the meantime.
pub struct GrammarBinding {
    pub label: &'static str,
    pub(crate) grammar: ParseLanguage,
}

/// The grammar bound to a detection label, if any frontend claims it. This
/// is the single label→grammar table; `ParseLanguage::from_label` delegates
/// here.
pub(crate) fn grammar_for_label(label: &str) -> Option<ParseLanguage> {
    all_frontends().iter().find_map(|frontend| {
        frontend
            .grammars
            .iter()
            .find(|binding| binding.label == label)
            .map(|binding| binding.grammar)
    })
}

/// Per-language import extraction, owned by the frontend. All functions
/// take the shared parse view so tree-based extractors reuse the one syntax
/// tree the audits already produced; text-based extractors read
/// `parsed.content()` and never parse.
pub struct ImportExtractor {
    /// Full import set — real coupling-graph edges.
    pub(crate) eager: fn(&ParsedFile) -> HashSet<String>,
    /// Edges that exist for coupling/fan-out but create no module-load
    /// dependency (Python function-body imports, TS type-only imports);
    /// cycle detection subtracts them. `None` when the language has none.
    pub(crate) deferred: Option<fn(&ParsedFile) -> HashSet<String>>,
    /// 1-indexed line spans per imported specifier, for edge evidence.
    pub(crate) spans: fn(&ParsedFile) -> BTreeMap<String, (usize, usize)>,
}

/// Static descriptor for one language (or dialect family) frontend.
pub struct LanguageFrontend {
    /// Stable slug; equals the primary knowledge-pack profile id.
    pub id: &'static str,
    /// Primary display label, as produced by language detection.
    pub label: &'static str,
    /// Context-classifier kind this frontend answers for.
    pub kind: LanguageKind,
    /// Knowledge-pack profile ids this frontend claims, dialects included
    /// (e.g. `typescript` claims `typescript-react`). Detection data —
    /// extensions, filenames, aliases — stays in the pack; the frontend
    /// references profiles instead of duplicating them.
    pub knowledge_ids: &'static [&'static str],
    /// Detection labels this frontend can parse, with their grammars.
    /// Empty for frontends without a bundled tree-sitter grammar.
    pub grammars: &'static [GrammarBinding],
    /// Import extraction, when the language has an extractor.
    pub(crate) imports: Option<&'static ImportExtractor>,
    /// Taint-lite tables (sources, sinks, sanitizers, grammar shapes), when
    /// the language participates in taint analysis.
    pub(crate) taint: Option<&'static crate::review::signals::taint::tables::TaintTables>,
    /// Boundary/algorithmic review-signal node-kind tables.
    pub(crate) review: Option<&'static crate::review::signals::tables::ReviewTables>,
    /// Runtime-risk audit tables (AST + line-scanner emitters) for the
    /// shared, generic engine. `None` for a language whose runtime-risk
    /// coverage instead lives in a dedicated, standalone audit — see
    /// [`dedicated_risk_audit`](Self::dedicated_risk_audit).
    pub(crate) risk:
        Option<&'static crate::audits::code_quality::language_risk::tables::RiskTables>,
    /// The rule id of a standalone, language-specific runtime-risk audit
    /// that satisfies the `RuntimeRisk` capability outside the shared
    /// engine — set when a language's runtime-risk logic is contextual
    /// enough (path-aware suppression, structural analysis) that forcing it
    /// into the generic per-node `RiskTables` shape would be churn without
    /// benefit. Rust's `language.rust.panic-risk` is the only one today.
    /// Documentation only; the audit itself is wired into the scan pipeline
    /// independently and does not run through this pointer.
    pub(crate) dedicated_risk_audit: Option<&'static str>,
    /// Path and naming conventions (test files, test support, entrypoints).
    pub(crate) conventions: &'static conventions::PathConventions,
    /// Framework detection probe, when the language frontend owns a probe.
    pub(crate) framework_probe:
        Option<fn(&std::path::Path) -> Vec<crate::frameworks::types::DetectedFramework>>,
}

/// Every registered frontend, including the generic fallback (last).
static ALL_FRONTENDS: [&LanguageFrontend; 9] = [
    &rust::RUST,
    &javascript::TYPESCRIPT,
    &javascript::JAVASCRIPT,
    &python::PYTHON,
    &go::GO,
    &java::JAVA,
    &csharp::CSHARP,
    &kotlin::KOTLIN,
    &generic::GENERIC,
];

pub fn all_frontends() -> &'static [&'static LanguageFrontend] {
    &ALL_FRONTENDS
}

/// The frontend responsible for `kind`. Kinds without a dedicated frontend
/// fall back to [`generic::GENERIC`]; the match is exhaustive so adding a
/// `LanguageKind` variant forces a routing decision here.
pub fn frontend_for_kind(kind: LanguageKind) -> &'static LanguageFrontend {
    match kind {
        LanguageKind::Rust => &rust::RUST,
        LanguageKind::TypeScript => &javascript::TYPESCRIPT,
        LanguageKind::JavaScript => &javascript::JAVASCRIPT,
        LanguageKind::Python => &python::PYTHON,
        LanguageKind::Go => &go::GO,
        LanguageKind::Java => &java::JAVA,
        LanguageKind::CSharp => &csharp::CSHARP,
        LanguageKind::Kotlin => &kotlin::KOTLIN,
        LanguageKind::Swift
        | LanguageKind::C
        | LanguageKind::Cpp
        | LanguageKind::CHeader
        | LanguageKind::Php
        | LanguageKind::Ruby
        | LanguageKind::Dart
        | LanguageKind::Scala
        | LanguageKind::Shell
        | LanguageKind::PowerShell
        | LanguageKind::Sql
        | LanguageKind::Html
        | LanguageKind::Css
        | LanguageKind::Scss
        | LanguageKind::Elixir
        | LanguageKind::Erlang
        | LanguageKind::Haskell
        | LanguageKind::OCaml
        | LanguageKind::FSharp
        | LanguageKind::R
        | LanguageKind::Julia
        | LanguageKind::Lua
        | LanguageKind::Perl
        | LanguageKind::Zig
        | LanguageKind::Solidity
        | LanguageKind::ObjectiveC
        | LanguageKind::Terraform
        | LanguageKind::Dockerfile
        | LanguageKind::Nix
        | LanguageKind::Json
        | LanguageKind::Toml
        | LanguageKind::Yaml
        | LanguageKind::Markdown
        | LanguageKind::Unknown => &generic::GENERIC,
    }
}

/// The frontend that claims a knowledge-pack profile id, if any.
pub fn frontend_for_knowledge_id(id: &str) -> Option<&'static LanguageFrontend> {
    all_frontends()
        .iter()
        .copied()
        .find(|frontend| frontend.knowledge_ids.contains(&id))
}

/// The frontend answering for a detection label (`FileFacts.language`),
/// matched via its display label or any of its grammar-binding labels.
pub(crate) fn frontend_for_label(label: &str) -> Option<&'static LanguageFrontend> {
    all_frontends().iter().copied().find(|frontend| {
        frontend.label == label
            || frontend
                .grammars
                .iter()
                .any(|binding| binding.label == label)
    })
}

/// The import extractor for a detection label, if the language has one.
pub(crate) fn imports_for_label(label: &str) -> Option<&'static ImportExtractor> {
    frontend_for_label(label).and_then(|frontend| frontend.imports)
}

/// The taint tables for a detection label, if the language participates in
/// taint analysis.
pub(crate) fn taint_for_label(
    label: &str,
) -> Option<&'static crate::review::signals::taint::tables::TaintTables> {
    frontend_for_label(label).and_then(|frontend| frontend.taint)
}

/// The review-signal tables for a detection label, if wired.
pub(crate) fn review_for_label(
    label: &str,
) -> Option<&'static crate::review::signals::tables::ReviewTables> {
    frontend_for_label(label).and_then(|frontend| frontend.review)
}

/// The removed-behavior recognizers claiming a file extension, if any. This
/// dispatch predates label-based detection; the extension lists live on the
/// tables verbatim and the guard tests keep them from drifting or colliding.
pub(crate) fn removed_for_extension(
    ext: &str,
) -> Option<&'static crate::review::signals::tables::RemovedTables> {
    all_frontends().iter().find_map(|frontend| {
        frontend
            .review
            .and_then(|review| review.removed)
            .filter(|removed| removed.extensions.contains(&ext))
    })
}
