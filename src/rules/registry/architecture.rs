use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;
use crate::rules::{RuleLifecycle, SignalSource};

pub(super) static RULES: &[RuleMetadata] = &[
    RuleMetadata {
        rule_id: "architecture.large-file",
        title: "File exceeds recommended size",
        category: FindingCategory::Architecture,
        default_severity: Severity::Medium,
        max_severity: Severity::High,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::TextHeuristic,
        docs_url: None,
        description: "This file has more lines of code than the configured threshold. Large files accumulate responsibilities over time and make navigation and testing harder.",
        recommendation: Some(
            "Split the file into smaller, focused modules. Use repopilot.toml `max_file_loc` to adjust the threshold.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "architecture.deep-directory-nesting",
        title: "Deep directory nesting detected",
        category: FindingCategory::Architecture,
        default_severity: Severity::Low,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::TextHeuristic,
        docs_url: None,
        description: "A file is nested deeply within the directory structure. Deep directory nesting makes the codebase harder to navigate, import from, and maintain.",
        recommendation: Some(
            "Simplify the directory structure or introduce path aliases to flatten the file layout. Adjust the max depth threshold using scan config.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "architecture.deep-relative-imports",
        title: "Deep relative import found",
        category: FindingCategory::Architecture,
        default_severity: Severity::Low,
        max_severity: Severity::Medium,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::TextHeuristic,
        docs_url: None,
        description: "A source file imports across three or more parent directories. Deep relative imports are fragile during refactors and often indicate missing module boundaries or aliases.",
        recommendation: Some(
            "Introduce a stable module boundary, path alias, or facade module instead of importing through multiple parent directories.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "architecture.barrel-file-risk",
        title: "Risky barrel file detected",
        category: FindingCategory::Architecture,
        default_severity: Severity::Low,
        default_confidence: Confidence::Low,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::TextHeuristic,
        docs_url: None,
        description: "An index file re-exports many modules or relies heavily on wildcard exports. Large barrel files can become unstable module hubs and make dependency boundaries harder to understand.",
        recommendation: Some(
            "Split the barrel into smaller feature-level entrypoints or replace wildcard exports with explicit, stable public APIs.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "architecture.too-many-modules",
        title: "Directory contains too many modules",
        category: FindingCategory::Architecture,
        default_severity: Severity::Medium,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::TextHeuristic,
        docs_url: None,
        description: "A directory contains more files than the configured threshold, suggesting it may need to be broken into sub-packages.",
        recommendation: Some(
            "Group related files into sub-directories. Adjust `max_directory_modules` in repopilot.toml if the current threshold is too strict.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "architecture.circular-dependency",
        title: "Circular import dependency detected",
        category: FindingCategory::Architecture,
        default_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Stable,
        signal_source: SignalSource::ImportGraph,
        docs_url: Some(
            "https://github.com/MykytaStel/repopilot/blob/main/docs/rulesets.md#architecture",
        ),
        description: "Two or more files import each other, forming a cycle. Circular dependencies make build order undefined, complicate testing, and prevent dead-code elimination.",
        recommendation: Some(
            "Extract the shared logic into a third module that both files can import without creating a cycle.",
        ),
        false_positive_notes: Some(
            "Generated, fixture, and test-only import cycles should stay out of default scans; production source cycles are treated as actionable.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "architecture.excessive-fan-out",
        title: "File imports too many project-internal modules",
        category: FindingCategory::Architecture,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Stable,
        signal_source: SignalSource::ImportGraph,
        docs_url: Some(
            "https://github.com/MykytaStel/repopilot/blob/main/docs/rulesets.md#architecture",
        ),
        description: "This file depends on an unusually large number of other internal modules, making it a fragile integration point that breaks whenever any dependency changes.",
        recommendation: Some(
            "Introduce an abstraction layer or facade to reduce the number of direct imports. Consider splitting responsibilities across multiple files.",
        ),
        false_positive_notes: Some(
            "Aggregator entrypoints can be acceptable when intentionally reviewed; default thresholds should be raised locally only after that decision.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "architecture.high-instability-hub",
        title: "High-instability hub: high fan-in and high fan-out",
        category: FindingCategory::Architecture,
        default_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Stable,
        signal_source: SignalSource::ImportGraph,
        docs_url: Some(
            "https://github.com/MykytaStel/repopilot/blob/main/docs/rulesets.md#architecture",
        ),
        description: "This file is both widely imported (high fan-in) and depends on many other modules (high fan-out). Changes here ripple across the codebase with no stable upstream to absorb them.",
        recommendation: Some(
            "Separate the stable, widely-imported interface from the volatile implementation details to reduce coupling.",
        ),
        false_positive_notes: Some(
            "Framework entrypoints and generated hubs can be expected; production modules with high fan-in and fan-out should be reviewed.",
        ),
        ..RuleMetadata::DEFAULT
    },
];
