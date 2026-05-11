use crate::findings::types::{FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;

pub(super) static RULES: &[RuleMetadata] = &[
    RuleMetadata {
        rule_id: "architecture.large-file",
        title: "File exceeds recommended size",
        category: FindingCategory::Architecture,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "This file has more lines of code than the configured threshold. Large files accumulate responsibilities over time and make navigation and testing harder.",
        recommendation: Some(
            "Split the file into smaller, focused modules. Use repopilot.toml `max_file_loc` to adjust the threshold.",
        ),
    },
    RuleMetadata {
        rule_id: "architecture.deep-nesting",
        title: "Directory nesting exceeds recommended depth",
        category: FindingCategory::Architecture,
        default_severity: Severity::Low,
        docs_url: None,
        description: "A directory path is deeper than the configured nesting threshold. Deeply nested structures make file discovery and import paths harder to maintain.",
        recommendation: Some(
            "Flatten the directory structure or consolidate related modules at a higher level.",
        ),
    },
    RuleMetadata {
        rule_id: "architecture.deep-relative-imports",
        title: "Deep relative import found",
        category: FindingCategory::Architecture,
        default_severity: Severity::Low,
        docs_url: None,
        description: "A source file imports across three or more parent directories. Deep relative imports are fragile during refactors and often indicate missing module boundaries or aliases.",
        recommendation: Some(
            "Introduce a stable module boundary, path alias, or facade module instead of importing through multiple parent directories.",
        ),
    },
    RuleMetadata {
        rule_id: "architecture.barrel-file-risk",
        title: "Risky barrel file detected",
        category: FindingCategory::Architecture,
        default_severity: Severity::Low,
        docs_url: None,
        description: "An index file re-exports many modules or relies heavily on wildcard exports. Large barrel files can become unstable module hubs and make dependency boundaries harder to understand.",
        recommendation: Some(
            "Split the barrel into smaller feature-level entrypoints or replace wildcard exports with explicit, stable public APIs.",
        ),
    },
    RuleMetadata {
        rule_id: "architecture.too-many-modules",
        title: "Directory contains too many modules",
        category: FindingCategory::Architecture,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "A directory contains more files than the configured threshold, suggesting it may need to be broken into sub-packages.",
        recommendation: Some(
            "Group related files into sub-directories. Adjust `max_directory_modules` in repopilot.toml if the current threshold is too strict.",
        ),
    },
    RuleMetadata {
        rule_id: "architecture.circular-dependency",
        title: "Circular import dependency detected",
        category: FindingCategory::Architecture,
        default_severity: Severity::High,
        docs_url: None,
        description: "Two or more files import each other, forming a cycle. Circular dependencies make build order undefined, complicate testing, and prevent dead-code elimination.",
        recommendation: Some(
            "Extract the shared logic into a third module that both files can import without creating a cycle.",
        ),
    },
    RuleMetadata {
        rule_id: "architecture.excessive-fan-out",
        title: "File imports too many project-internal modules",
        category: FindingCategory::Architecture,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "This file depends on an unusually large number of other internal modules, making it a fragile integration point that breaks whenever any dependency changes.",
        recommendation: Some(
            "Introduce an abstraction layer or facade to reduce the number of direct imports. Consider splitting responsibilities across multiple files.",
        ),
    },
    RuleMetadata {
        rule_id: "architecture.high-instability-hub",
        title: "High-instability hub: high fan-in and high fan-out",
        category: FindingCategory::Architecture,
        default_severity: Severity::High,
        docs_url: None,
        description: "This file is both widely imported (high fan-in) and depends on many other modules (high fan-out). Changes here ripple across the codebase with no stable upstream to absorb them.",
        recommendation: Some(
            "Separate the stable, widely-imported interface from the volatile implementation details to reduce coupling.",
        ),
    },
];
