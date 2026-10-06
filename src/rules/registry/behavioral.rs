use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;
use crate::rules::{RuleLifecycle, RuleRequirements, SignalSource};

pub(super) static RULES: &[RuleMetadata] = &[
    RuleMetadata {
        rule_id: "behavioral.removed-export-still-imported",
        title: "Removed export is still imported",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::High,
        max_severity: Severity::High,
        default_confidence: Confidence::High,
        max_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,
        requirements: RuleRequirements::change_set_symbol_graph(RuleLifecycle::Preview),
        docs_url: Some("https://github.com/MykytaStel/repopilot/blob/main/docs/rules-reference.md"),
        description: "A changed TypeScript or JavaScript module removed a named or direct default export while a surviving direct local caller still imports that symbol from the same resolved module. Rust additionally covers removed public free functions imported or directly called through a file-backed child module declared in the caller.",
        recommendation: Some(
            "Restore the removed export or update every surviving caller, then run the repository's declared type-check, build, or focused tests.",
        ),
        false_positive_notes: Some(
            "JS/TS requires direct relative named or default imports with exact resolver proof. Rust requires an unambiguous plain child-module declaration in the caller; crate paths require src/lib.rs or src/main.rs. Rust attributes/cfg, macros, inline modules, forwarding, grouped/glob imports, arbitrary sibling paths, associated methods and generic-function call sites are outside this slice. Namespace imports, aliases, packages, dynamic/CommonJS forms, deep re-exports, file renames, deleted exporters, unsupported languages, and incomplete AST evidence are intentionally outside this claim.",
        ),
        tags: &[
            "behavioral",
            "api-contract",
            "typescript",
            "javascript",
            "rust",
        ],
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "behavioral.rust-public-function-arity-changed",
        title: "Rust public function arity changed",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::High,
        max_severity: Severity::High,
        default_confidence: Confidence::High,
        max_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,
        requirements: RuleRequirements::change_set_symbol_graph(RuleLifecycle::Preview),
        docs_url: Some("https://github.com/MykytaStel/repopilot/blob/main/docs/rules-reference.md"),
        description: "A changed Rust module changed the parameter count of a top-level public free function while an unambiguous direct local call still passes the previous count of arguments.",
        recommendation: Some(
            "Update the proven direct caller to match the new public function arity, or restore the previous signature, then run the Rust type-check or focused tests.",
        ),
        false_positive_notes: Some(
            "Rust support is limited to non-generic top-level pub fn declarations and direct module-qualified calls through an unambiguous file-backed child module declared in the caller. The caller's current argument count must match the previous function parameter count. Bare imported calls, macros, attributes/cfg, generic or variadic functions, methods, ambiguous module proof, and same-arity type changes are outside this slice.",
        ),
        tags: &["behavioral", "api-contract", "rust"],
        ..RuleMetadata::DEFAULT
    },
];
