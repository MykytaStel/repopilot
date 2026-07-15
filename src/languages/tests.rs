//! Guard tests that hold the language frontend contract together:
//! grammars claimed exactly once, bindings in lockstep with parse dispatch,
//! knowledge-pack ids resolvable, and a shrinking ledger of languages whose
//! declared support level exceeds what the wiring justifies.

use super::capability::{computed_support, support_rank};
use super::*;
use crate::knowledge::active_knowledge;
use crate::knowledge::language::{language_kind_from_id, profile_by_id};
use crate::knowledge::model::SupportLevel;
use std::collections::{BTreeMap, BTreeSet};

/// Forces this list to grow when a `ParseLanguage` variant is added: the
/// match below stops compiling until the new grammar is routed here and
/// claimed by a frontend.
const ALL_GRAMMARS: [ParseLanguage; 9] = [
    ParseLanguage::Rust,
    ParseLanguage::TypeScript,
    ParseLanguage::Tsx,
    ParseLanguage::JavaScript,
    ParseLanguage::Python,
    ParseLanguage::Go,
    ParseLanguage::Java,
    ParseLanguage::CSharp,
    ParseLanguage::Kotlin,
];

#[allow(dead_code)]
fn grammar_list_is_exhaustive(grammar: ParseLanguage) {
    match grammar {
        ParseLanguage::Rust
        | ParseLanguage::TypeScript
        | ParseLanguage::Tsx
        | ParseLanguage::JavaScript
        | ParseLanguage::Python
        | ParseLanguage::Go
        | ParseLanguage::Java
        | ParseLanguage::CSharp
        | ParseLanguage::Kotlin => {}
    }
}

/// The pinned label→grammar vocabulary. The registry is the only
/// label→grammar table (`ParseLanguage::from_label` delegates to it), so
/// this pin is what keeps an accidental binding edit — a dropped label or a
/// dialect pointed at the wrong grammar — from silently changing parse
/// behavior. Extend it deliberately when a frontend gains a label.
const PINNED_LABEL_GRAMMARS: [(&str, ParseLanguage); 11] = [
    ("Rust", ParseLanguage::Rust),
    ("TypeScript", ParseLanguage::TypeScript),
    ("TypeScript React", ParseLanguage::Tsx),
    ("JavaScript", ParseLanguage::JavaScript),
    ("JavaScript React", ParseLanguage::JavaScript),
    ("Python", ParseLanguage::Python),
    ("Go", ParseLanguage::Go),
    ("Java", ParseLanguage::Java),
    ("CSharp", ParseLanguage::CSharp),
    ("C#", ParseLanguage::CSharp),
    ("Kotlin", ParseLanguage::Kotlin),
];

#[test]
fn every_grammar_is_claimed_by_exactly_one_frontend() {
    let mut owners: BTreeMap<&'static str, BTreeSet<&'static str>> = BTreeMap::new();
    for frontend in all_frontends() {
        for binding in frontend.grammars {
            owners
                .entry(grammar_name(binding.grammar))
                .or_default()
                .insert(frontend.id);
        }
    }

    for grammar in ALL_GRAMMARS {
        let claimants = owners
            .get(grammar_name(grammar))
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            claimants.len(),
            1,
            "grammar {:?} must be claimed by exactly one frontend, got {claimants:?}",
            grammar
        );
    }
}

#[test]
fn grammar_label_vocabulary_is_pinned() {
    for (label, grammar) in PINNED_LABEL_GRAMMARS {
        assert_eq!(
            grammar_for_label(label),
            Some(grammar),
            "label '{label}' no longer resolves to the pinned grammar"
        );
    }

    let bound_labels: BTreeSet<&str> = all_frontends()
        .iter()
        .flat_map(|frontend| frontend.grammars.iter().map(|binding| binding.label))
        .collect();
    let pinned: BTreeSet<&str> = PINNED_LABEL_GRAMMARS
        .into_iter()
        .map(|(label, _)| label)
        .collect();
    assert_eq!(
        bound_labels, pinned,
        "registry grammar labels drifted from the pinned vocabulary"
    );
}

#[test]
fn knowledge_ids_resolve_and_route_back_to_their_frontend() {
    for frontend in all_frontends() {
        for id in frontend.knowledge_ids {
            let profile = profile_by_id(id);
            assert!(
                profile.is_some(),
                "frontend '{}' claims knowledge id '{id}' missing from the bundled pack",
                frontend.id
            );
            assert_eq!(
                language_kind_from_id(id),
                frontend.kind,
                "knowledge id '{id}' classifies to a different kind than frontend '{}'",
                frontend.id
            );
            assert_eq!(
                frontend_for_knowledge_id(id).map(|found| found.id),
                Some(frontend.id),
                "knowledge id '{id}' is claimed by more than one frontend"
            );
        }
    }
}

#[test]
fn kinds_route_to_their_frontends_and_the_rest_to_generic() {
    for frontend in all_frontends() {
        if frontend.id == "generic" {
            continue;
        }
        assert_eq!(frontend_for_kind(frontend.kind).id, frontend.id);
    }

    for kind in [
        LanguageKind::Swift,
        LanguageKind::Cpp,
        LanguageKind::Terraform,
        LanguageKind::Markdown,
        LanguageKind::Unknown,
    ] {
        assert_eq!(frontend_for_kind(kind).id, "generic");
    }
}

/// Pins which frontends own an import extractor and which deferred-edge
/// semantics they register, so a migration cannot silently drop a language
/// from the coupling graph (or grow deferred behavior a language never had).
#[test]
fn import_extractor_coverage_is_pinned() {
    let with_deferred: BTreeSet<&str> =
        ["typescript", "javascript", "python"].into_iter().collect();
    let without_imports: BTreeSet<&str> = ["csharp", "generic"].into_iter().collect();

    for frontend in all_frontends() {
        match frontend.imports {
            Some(extractor) => {
                assert!(
                    !without_imports.contains(frontend.id),
                    "frontend '{}' unexpectedly gained an import extractor; update the pin",
                    frontend.id
                );
                assert_eq!(
                    extractor.deferred.is_some(),
                    with_deferred.contains(frontend.id),
                    "deferred-import semantics changed for frontend '{}'",
                    frontend.id
                );
            }
            None => assert!(
                without_imports.contains(frontend.id),
                "frontend '{}' lost its import extractor",
                frontend.id
            ),
        }
    }
}

/// Pins which frontends participate in taint-lite. The gate used to be the
/// private `TaintLang::from_label` enum; now it is the frontend's `taint`
/// field, and this pin keeps a refactor from silently adding or removing a
/// language from taint analysis.
#[test]
fn taint_participation_is_pinned() {
    let with_taint: BTreeSet<&str> = ["typescript", "javascript", "python", "go"]
        .into_iter()
        .collect();
    for frontend in all_frontends() {
        assert_eq!(
            frontend.taint.is_some(),
            with_taint.contains(frontend.id),
            "taint participation changed for frontend '{}'",
            frontend.id
        );
    }
}

/// The honesty ledger. Languages the bundled pack declares `rule-aware`
/// whose frontends do not yet justify it. Migration PRs shrink this set by
/// wiring capabilities (or PR-9 downgrades over-claimed pack declarations —
/// `c`/`cpp` have no grammar at all and are the first candidates). It must
/// never grow.
#[test]
fn declared_rule_aware_support_gap_ledger_only_shrinks() {
    let expected_gaps: BTreeSet<&str> = [
        "c",
        "cpp",
        "csharp",
        "go",
        "java",
        "javascript",
        "kotlin",
        "python",
        "rust",
        "typescript",
    ]
    .into_iter()
    .collect();

    let mut gaps = BTreeSet::new();
    for profile in &active_knowledge().languages {
        if profile.support != SupportLevel::RuleAware {
            continue;
        }
        let computed = frontend_for_knowledge_id(&profile.id)
            .map(computed_support)
            .unwrap_or(SupportLevel::DetectOnly);
        if support_rank(computed) < support_rank(SupportLevel::RuleAware) {
            gaps.insert(profile.id.as_str());
        }
    }

    assert_eq!(
        gaps, expected_gaps,
        "rule-aware support gaps changed; shrink the ledger when wiring capabilities, \
         never grow it"
    );
}

fn grammar_name(grammar: ParseLanguage) -> &'static str {
    match grammar {
        ParseLanguage::Rust => "rust",
        ParseLanguage::TypeScript => "typescript",
        ParseLanguage::Tsx => "tsx",
        ParseLanguage::JavaScript => "javascript",
        ParseLanguage::Python => "python",
        ParseLanguage::Go => "go",
        ParseLanguage::Java => "java",
        ParseLanguage::CSharp => "csharp",
        ParseLanguage::Kotlin => "kotlin",
    }
}
