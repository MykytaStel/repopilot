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
    let without_imports: BTreeSet<&str> = ["generic"].into_iter().collect();

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
    let with_taint: BTreeSet<&str> = ["typescript", "javascript", "python", "go", "java", "csharp"]
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

/// Pins which frontends carry review-signal tables and whether their AST
/// boundary classification is wired. Every table-carrying frontend now has
/// boundary wired (C#'s historical dead-arm exception was closed by the
/// honesty pass).
#[test]
fn review_table_coverage_is_pinned() {
    let with_review: BTreeSet<&str> = [
        "rust",
        "typescript",
        "javascript",
        "python",
        "go",
        "java",
        "kotlin",
        "csharp",
    ]
    .into_iter()
    .collect();
    let without_boundary: BTreeSet<&str> = BTreeSet::new();

    for frontend in all_frontends() {
        match frontend.review {
            Some(tables) => {
                assert!(
                    with_review.contains(frontend.id),
                    "frontend '{}' unexpectedly gained review tables; update the pin",
                    frontend.id
                );
                assert_eq!(
                    tables.boundary.is_none(),
                    without_boundary.contains(frontend.id),
                    "boundary wiring changed for frontend '{}'",
                    frontend.id
                );
            }
            None => assert!(
                !with_review.contains(frontend.id),
                "frontend '{}' lost its review tables",
                frontend.id
            ),
        }
    }
}

/// Pins the removed-behavior recognizer coverage: which frontends carry one
/// and the exact extension vocabulary, and that no extension is claimed
/// twice. `cts` is knowingly unreachable (detection never labels it) —
/// preserved verbatim from the pre-registry dispatch, not an endorsement.
#[test]
fn removed_recognizer_extensions_are_pinned() {
    let expected: BTreeMap<&str, usize> = [
        ("js", 2),
        ("mjs", 2),
        ("cjs", 2),
        ("ts", 2),
        ("mts", 2),
        ("cts", 2),
        ("tsx", 2),
        ("jsx", 2),
        ("py", 1),
        ("go", 1),
        ("rs", 1),
        ("java", 1),
        ("kt", 1),
        ("kts", 1),
        ("cs", 1),
    ]
    .into_iter()
    .collect();

    let mut claims: BTreeMap<&str, usize> = BTreeMap::new();
    for frontend in all_frontends() {
        let Some(removed) = frontend.review.and_then(|review| review.removed) else {
            continue;
        };
        for ext in removed.extensions {
            *claims.entry(ext).or_default() += 1;
        }
    }

    // The JS family table is shared by the typescript and javascript
    // frontends, so its extensions are claimed twice — by the same static.
    // Everything else must be claimed exactly once.
    assert_eq!(
        claims, expected,
        "removed-recognizer extension vocabulary drifted"
    );

    for ext in expected.keys() {
        assert!(
            crate::languages::removed_for_extension(ext).is_some(),
            "extension '{ext}' lost its removed-behavior recognizer"
        );
    }
}

/// Pins which frontends carry runtime-risk tables. Rust intentionally has
/// none here: its dedicated `rust_panic_risk` audit is a separate rule
/// family, and how it counts toward the capability model is a decision for
/// the honesty pass, not a refactor default.
#[test]
fn runtime_risk_participation_is_pinned() {
    let with_risk: BTreeSet<&str> = [
        "typescript",
        "javascript",
        "python",
        "go",
        "java",
        "kotlin",
        "csharp",
    ]
    .into_iter()
    .collect();
    for frontend in all_frontends() {
        assert_eq!(
            frontend.risk.is_some(),
            with_risk.contains(frontend.id),
            "runtime-risk participation changed for frontend '{}'",
            frontend.id
        );
    }
}

/// Pins the convention surface: the Rust `test_` prefix opt-out, which
/// frontends carry a test-support recognizer (and its evidence reason), and
/// which carry an entrypoint content probe.
#[test]
fn path_conventions_are_pinned() {
    use super::conventions::all_conventions;

    for (id, conventions) in all_conventions() {
        assert_eq!(
            conventions.test_prefix_marks_test,
            id != "rust",
            "test_ prefix convention changed for frontend '{id}'"
        );

        let expected_support_reason = match id {
            "rust" => Some("recognized Rust test-support helper filename"),
            "java" | "kotlin" | "csharp" => {
                Some("recognized managed-language test-support source-set path")
            }
            _ => None,
        };
        assert_eq!(
            conventions.test_support.map(|support| support.reason),
            expected_support_reason,
            "test-support convention changed for frontend '{id}'"
        );

        let expects_entry_probe = matches!(id, "rust" | "python" | "go");
        assert_eq!(
            conventions.entrypoint_content.is_some(),
            expects_entry_probe,
            "entrypoint content probe changed for frontend '{id}'"
        );
    }
}

/// The honesty ledger. Languages the bundled pack declares `rule-aware`
/// whose unified frontend wiring does not fully justify it. It must never
/// grow. After Phase A1 it names exactly one:
///
/// - **rust** — its runtime-risk coverage is the dedicated `rust.panic-risk`
///   audit, which lives outside the shared frontend `risk` table, so the
///   capability model does not count it. Real coverage exists; the
///   accounting is the gap (closed by A2).
///
/// `c`/`cpp` left the ledger by an honest pack downgrade (no grammar, no
/// frontend); `csharp` completed its contract (imports, taint, boundary) in
/// A1; every other rule-aware language computes through the contract.
#[test]
fn declared_rule_aware_support_gap_ledger_only_shrinks() {
    let expected_gaps: BTreeSet<&str> = ["rust"].into_iter().collect();

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
