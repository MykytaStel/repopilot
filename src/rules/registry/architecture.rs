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
        // Demoted to Medium when the repository has unresolved relative
        // imports: fan-in is then a lower bound and instability is overstated.
        contextual_confidence: true,
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
            "Framework entrypoints and generated hubs can be expected; production modules with high fan-in and fan-out should be reviewed. When the repository contains unresolved relative imports, fan-in is a lower bound and the finding is reported at Medium confidence.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "architecture.dead-module",
        title: "Dead module detected",
        category: FindingCategory::Architecture,
        default_severity: Severity::Low,
        default_confidence: Confidence::High,
        // Demoted to Low when the repository has unresolved internal imports
        // (the graph is provably incomplete); suppressed entirely when an
        // unresolved import could plausibly target the candidate file.
        contextual_confidence: true,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::ImportGraph,
        docs_url: None,
        description: "This production file is not imported by any other project file and is not a known entrypoint. It may be dead code.",
        recommendation: Some(
            "Remove the file if it is no longer used, or ensure it is exported in the package's public API.",
        ),
        false_positive_notes: Some(
            "Dynamic imports, dependency injection, and build-tool aliases create importers the graph cannot see. When the repository contains unresolved internal imports (relative paths, aliases, or workspace-package imports the resolver cannot wire up — common in monorepos) the finding is reported at Low confidence, and it is suppressed when an unresolved import matches the candidate file's name.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "architecture.layer-violation",
        title: "Layer violation detected",
        category: FindingCategory::Architecture,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::ImportGraph,
        docs_url: None,
        description: "A module imports another module from a higher layer than its own, against the order declared in `[[architecture.layers]]`. Opt-in: emits nothing unless layers are configured.",
        recommendation: Some(
            "Invert the dependency using an interface, or move the shared logic into a layer at or below the importer.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "architecture.test-leak",
        title: "Test code leaked into production",
        category: FindingCategory::Architecture,
        // Medium while Experimental and undocumented; a High default would
        // require a `docs_url` per the finding contract (rules reference: PR-J).
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::ImportGraph,
        docs_url: None,
        description: "A production module imports a test or fixture module.",
        recommendation: Some(
            "Remove the test dependency from production code or extract the shared logic into a production utility.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "architecture.package-boundary-violation",
        title: "Package boundary violation",
        category: FindingCategory::Architecture,
        default_severity: Severity::Medium,
        // Glob-driven (`package_roots`) findings keep the Medium default;
        // manifest-driven findings on a detected workspace are raised to the
        // High ceiling because the boundary is declared by the repo itself.
        default_confidence: Confidence::Medium,
        max_confidence: Confidence::High,
        contextual_confidence: true,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::ImportGraph,
        docs_url: None,
        description: "A file imports a private module from another package instead of using its public API. Auto-enabled on a detected npm/pnpm/Cargo/Go workspace; can also be driven explicitly with `[architecture] package_roots`.",
        recommendation: Some(
            "Import from the package's public barrel file (e.g. index.ts, mod.rs) instead of reaching into its internal implementation.",
        ),
        false_positive_notes: Some(
            "Public entry files (index.ts/js/tsx/jsx, mod.rs, lib.rs) are exempt, as are imports within the same package. Intentional deep imports in a private monorepo can be suppressed via feedback or by not configuring package boundaries. Workspace-detected boundaries report at High confidence; configured `package_roots` globs report at the Medium default.",
        ),
        ..RuleMetadata::DEFAULT
    },
];
