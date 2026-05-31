use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;
use crate::rules::{RuleLifecycle, SignalSource};

pub(super) static RULES: &[RuleMetadata] = &[
    RuleMetadata {
        rule_id: "language.rust.panic-risk",
        title: "Risky Rust panic or unwrap usage",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::TextHeuristic,
        docs_url: None,
        description: "Rust panic-style operations such as unwrap(), expect(), panic!, todo!, and unimplemented! can be risky in reusable production code. Their severity depends on whether the code is test code, CLI boundary code, library code, or domain code.",
        recommendation: Some(
            "Use context-aware error handling. Prefer Result, ?, typed errors, validation, or explicit fallback behavior in production and library code.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "language.go.panic-exit-risk",
        title: "Risky Go panic or process exit usage",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::TextHeuristic,
        docs_url: None,
        description: "Go panic and process-exit operations can terminate the program abruptly. Their risk depends on whether the file is test code, CLI boundary code, library code, or domain code.",
        recommendation: Some(
            "Return errors from reusable code and reserve panic/log.Fatal/os.Exit for narrow process boundaries where callers cannot recover.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "language.python.exception-risk",
        title: "Risky Python exception or assertion pattern",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,
        docs_url: None,
        description: "Broad exception handlers, production asserts, and NotImplementedError placeholders can hide failures or ship incomplete behaviour. Their severity depends on test, script, and domain context.",
        recommendation: Some(
            "Catch specific exceptions, use explicit runtime validation, and replace placeholders before production release.",
        ),
        false_positive_notes: Some(
            "Matches come from the parsed syntax tree (bare `except` clauses, `assert` statements, and `NotImplementedError`), so the same tokens inside comments or string literals are not flagged. A line heuristic with text-heuristic provenance is used only when the file fails to parse.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "language.javascript.runtime-exit-risk",
        title: "Risky JavaScript runtime exit or library throw",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,
        docs_url: None,
        description: "Process exits and generic thrown errors have different risk at a CLI boundary than in reusable browser, Node, or package code.",
        recommendation: Some(
            "Prefer returning typed errors, rejecting promises with actionable context, or centralising CLI exit handling at the entrypoint.",
        ),
        false_positive_notes: Some(
            "`process.exit` calls and `throw new Error(...)` are matched from the parsed syntax tree, so the same text inside comments or string literals is not flagged. A line heuristic with text-heuristic provenance is used only when the file fails to parse.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "language.managed.fatal-exception-risk",
        title: "Risky JVM or .NET fatal exception placeholder",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::TextHeuristic,
        docs_url: None,
        description: "Generic fatal exceptions and not-implemented placeholders in Java, Kotlin, or C# domain/library code can become runtime failures that callers cannot handle precisely.",
        recommendation: Some(
            "Replace placeholders with implemented behaviour and use domain-specific exception or result types where callers need to recover.",
        ),
        ..RuleMetadata::DEFAULT
    },
];
