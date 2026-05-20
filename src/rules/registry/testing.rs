use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;
use crate::rules::{RuleLifecycle, SignalSource};

pub(super) static RULES: &[RuleMetadata] = &[
    RuleMetadata {
        rule_id: "testing.missing-test-folder",
        title: "No test directory found in project",
        category: FindingCategory::Testing,
        default_severity: Severity::Medium,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::TextHeuristic,
        docs_url: None,
        description: "The project has no recognisable test directory (tests/, __tests__, spec/). Without tests, correctness can only be verified manually.",
        recommendation: Some(
            "Create a test directory and add at least smoke tests for the core logic.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "testing.source-without-test",
        title: "Source file has no corresponding test file",
        category: FindingCategory::Testing,
        default_severity: Severity::Low,
        default_confidence: Confidence::Low,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::TextHeuristic,
        docs_url: None,
        description: "A source file has no matching test file. Untested code is more likely to regress during refactoring.",
        recommendation: Some(
            "Add a test file alongside the source file, or co-locate tests within the same file following the project's testing conventions.",
        ),
        ..RuleMetadata::DEFAULT
    },
];
