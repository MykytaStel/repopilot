use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;
use crate::rules::{RuleLifecycle, RuleRequirements, SignalSource};

pub(super) static RULES: &[RuleMetadata] = &[RuleMetadata {
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
    description: "A changed TypeScript or JavaScript module removed a named export while a surviving direct local caller still imports that symbol from the same resolved module.",
    recommendation: Some(
        "Restore the removed export or update every surviving caller, then run the repository's declared type-check, build, or focused tests.",
    ),
    false_positive_notes: Some(
        "Only direct relative named imports with exact resolver proof are reported. Default and namespace imports, aliases, packages, dynamic/CommonJS forms, deep re-exports, file renames, deleted exporters, unsupported languages, and incomplete AST/cache evidence are intentionally outside this claim.",
    ),
    tags: &["behavioral", "api-contract", "typescript", "javascript"],
    ..RuleMetadata::DEFAULT
}];
