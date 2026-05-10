use crate::findings::types::{FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;

pub(super) static RULES: &[RuleMetadata] = &[
    RuleMetadata {
        rule_id: "code-quality.complex-file",
        title: "File has high cyclomatic complexity",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "The file's branch count density exceeds the complexity threshold, indicating too many execution paths. High complexity increases the defect rate and testing burden.",
        recommendation: Some(
            "Extract conditionals into well-named helper functions. Prefer early returns to deeply nested if/else chains.",
        ),
    },
    RuleMetadata {
        rule_id: "code-quality.long-function",
        title: "Function body exceeds recommended length",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "A function is longer than the configured line threshold. Long functions are harder to test, understand, and safely refactor.",
        recommendation: Some(
            "Break the function into smaller, single-purpose helpers. Aim for functions that fit on a single screen.",
        ),
    },
    RuleMetadata {
        rule_id: "code-marker.todo",
        title: "TODO marker found",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Low,
        docs_url: None,
        description: "A TODO comment marks unfinished work. Unresolved TODOs accumulate as technical debt if not tracked in an issue tracker.",
        recommendation: Some(
            "Convert the TODO into a tracked issue and reference the issue number in the comment, or resolve it immediately.",
        ),
    },
    RuleMetadata {
        rule_id: "code-marker.fixme",
        title: "FIXME marker found",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "A FIXME comment marks known broken or problematic code that has not yet been addressed.",
        recommendation: Some(
            "Fix the issue or file a bug report and reference its number in the comment.",
        ),
    },
    RuleMetadata {
        rule_id: "code-marker.hack",
        title: "HACK marker found",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "A HACK comment marks a workaround that bypasses a proper solution. Hacks tend to become permanent and break under refactoring.",
        recommendation: Some(
            "Document why the hack exists and create a tracked issue to replace it with a proper implementation.",
        ),
    },
];
