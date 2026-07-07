use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;
use crate::rules::{RuleLifecycle, RuleRequirements, SignalSource};

pub(super) static RULES: &[RuleMetadata] = &[
    RuleMetadata {
        rule_id: "code-quality.complex-file",
        title: "File has high cyclomatic complexity",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        max_severity: Severity::High,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::TextHeuristic,

        requirements: RuleRequirements::file_text(RuleLifecycle::Preview),
        docs_url: None,
        description: "The file's branch count density exceeds the complexity threshold, indicating too many execution paths. High complexity increases the defect rate and testing burden.",
        recommendation: Some(
            "Extract conditionals into well-named helper functions. Prefer early returns to deeply nested if/else chains.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "code-quality.long-function",
        title: "Function body exceeds recommended length",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        contextual_confidence: true,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,

        requirements: RuleRequirements::file_ast(RuleLifecycle::Preview),
        docs_url: None,
        description: "A function is longer than the configured line threshold. Long functions are harder to test, understand, and safely refactor.",
        recommendation: Some(
            "Break the function into smaller, single-purpose helpers. Aim for functions that fit on a single screen.",
        ),
        false_positive_notes: Some(
            "Parseable languages (Rust, TypeScript/JavaScript, Python) measure real function spans from the syntax tree. Languages without a tree-sitter grammar (Go, Java, C#, Kotlin) fall back to a line/brace heuristic and are reported with text-heuristic provenance.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "code-marker.todo",
        title: "TODO marker found",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Low,
        default_confidence: Confidence::Low,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::TextHeuristic,

        requirements: RuleRequirements::file_text(RuleLifecycle::Experimental),
        docs_url: None,
        description: "A TODO comment marks unfinished work. Unresolved TODOs accumulate as technical debt if not tracked in an issue tracker.",
        recommendation: Some(
            "Convert the TODO into a tracked issue and reference the issue number in the comment, or resolve it immediately.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "code-marker.fixme",
        title: "FIXME marker found",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        default_confidence: Confidence::Low,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::TextHeuristic,

        requirements: RuleRequirements::file_text(RuleLifecycle::Experimental),
        docs_url: None,
        description: "A FIXME comment marks known broken or problematic code that has not yet been addressed.",
        recommendation: Some(
            "Fix the issue or file a bug report and reference its number in the comment.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "code-marker.hack",
        title: "HACK marker found",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        default_confidence: Confidence::Low,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::TextHeuristic,

        requirements: RuleRequirements::file_text(RuleLifecycle::Experimental),
        docs_url: None,
        description: "A HACK comment marks a workaround that bypasses a proper solution. Hacks tend to become permanent and break under refactoring.",
        recommendation: Some(
            "Document why the hack exists and create a tracked issue to replace it with a proper implementation.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "code-quality.complex-function",
        title: "Function has high cognitive complexity",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,

        requirements: RuleRequirements::file_ast(RuleLifecycle::Preview),
        docs_url: None,
        description: "A function's control flow is deeply nested or branch-heavy, measured by a cognitive-complexity score that weights nesting depth rather than counting branches flatly. Deeply nested logic is disproportionately hard to read, test, and change.",
        recommendation: Some(
            "Reduce nesting: extract nested blocks into well-named helper functions and prefer early returns/guard clauses over deep if/else and loop nesting.",
        ),
        false_positive_notes: Some(
            "A wide but flat dispatcher (a large switch/match with shallow arms) scores low by design, so it is not flagged. Nested closures/callbacks are scored independently rather than folded into their enclosing function. Cross-reference: `code-quality.complex-file` counts branches per file and does not weight nesting.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "code-quality.deep-control-flow",
        title: "Deep control flow nesting detected",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Low,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,

        requirements: RuleRequirements::file_ast(RuleLifecycle::Preview),
        docs_url: None,
        description: "A source file contains deeply nested control flow blocks (if, loops, match, try). High nesting depth makes code hard to read, maintain, and test.",
        recommendation: Some(
            "Extract nested blocks into separate helper functions or simplify the control flow.",
        ),
        ..RuleMetadata::DEFAULT
    },
];
