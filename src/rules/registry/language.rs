use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;
use crate::rules::{RuleLifecycle, RuleRequirements, SignalSource};

pub(super) static RULES: &[RuleMetadata] = &[
    RuleMetadata {
        rule_id: "language.rust.panic-risk",
        title: "Risky Rust panic or unwrap usage",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        max_severity: Severity::High,
        default_confidence: Confidence::High,
        contextual_confidence: true,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,

        requirements: RuleRequirements::file_ast(RuleLifecycle::Preview),
        docs_url: None,
        description: "Rust panic-style operations such as unwrap(), expect(), panic!, todo!, and unimplemented! can be risky in reusable production code. Their severity depends on whether the code is test code, CLI boundary code, library code, or domain code.",
        recommendation: Some(
            "Use context-aware error handling. Prefer Result, ?, typed errors, validation, or explicit fallback behavior in production and library code.",
        ),
        false_positive_notes: Some(
            "`unwrap`/`expect`/`unwrap_err`/`expect_err` calls and `panic!`/`todo!`/`unimplemented!` macros are matched from the parsed syntax tree, so the same tokens inside comments or string literals (including multi-line raw strings) are not flagged, and infallible `write!`/`writeln!` result unwraps in report renderers are recognized structurally. Severity context (test/CLI/library/domain) and external-input escalation remain heuristic. A line scanner with text-heuristic provenance is used only when the file fails to parse.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "language.go.panic-exit-risk",
        title: "Risky Go panic or process exit usage",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        max_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,

        requirements: RuleRequirements::file_ast(RuleLifecycle::Preview),
        docs_url: None,
        description: "Go panic and process-exit operations can terminate the program abruptly. Their risk depends on whether the file is test code, CLI boundary code, library code, or domain code.",
        recommendation: Some(
            "Return errors from reusable code and reserve panic/log.Fatal/os.Exit for narrow process boundaries where callers cannot recover.",
        ),
        false_positive_notes: Some(
            "`panic`, `log.Fatal`/`log.Fatalf`, and `os.Exit` calls are matched from the parsed syntax tree, so the same text inside comments or string literals is not flagged. A line heuristic with text-heuristic provenance is used only when the file fails to parse.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "language.python.exception-risk",
        title: "Risky Python exception or assertion pattern",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        max_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,

        requirements: RuleRequirements::file_ast(RuleLifecycle::Preview),
        docs_url: None,
        description: "A broad `except:` handler can hide unrelated failures. `assert` (often type-narrowing or an internal invariant) and `raise NotImplementedError` (usually an abstract-method declaration) are overwhelmingly intentional, so they are kept low and surface only under the strict profile.",
        recommendation: Some(
            "Catch specific exceptions so unrelated failures are not hidden; use explicit runtime validation where an invariant must hold in production.",
        ),
        false_positive_notes: Some(
            "Matches come from the parsed syntax tree (bare `except` clauses, `assert` statements, and `NotImplementedError`), so the same tokens inside comments or string literals are not flagged. `assert` and `NotImplementedError` are downgraded to low (hidden by default, shown in `--profile strict`). A line heuristic with text-heuristic provenance is used only when the file fails to parse.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "language.javascript.runtime-exit-risk",
        title: "Risky JavaScript runtime exit",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        max_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,

        requirements: RuleRequirements::file_ast(RuleLifecycle::Preview),
        docs_url: None,
        description: "A `process.exit(...)` call terminates the host process, which is expected at a CLI boundary but unsafe in reusable browser, Node, or package code.",
        recommendation: Some(
            "Return typed errors or reject promises with actionable context from reusable modules, and centralise process exits at the CLI entrypoint.",
        ),
        false_positive_notes: Some(
            "`process.exit` calls are matched from the parsed syntax tree, so the same text inside comments or string literals is not flagged. A line heuristic with text-heuristic provenance is used only when the file fails to parse.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "language.managed.fatal-exception-risk",
        title: "Risky JVM or .NET fatal exception placeholder",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        max_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,

        requirements: RuleRequirements::file_ast(RuleLifecycle::Preview),
        docs_url: None,
        description: "Generic fatal exceptions and not-implemented placeholders in Java, Kotlin, or C# domain/library code can become runtime failures that callers cannot handle precisely.",
        recommendation: Some(
            "Replace placeholders with implemented behaviour and use domain-specific exception or result types where callers need to recover.",
        ),
        false_positive_notes: Some(
            "Fatal exceptions and placeholder throw/TODO structures are matched from the parsed syntax tree, so the same text inside comments or string literals is not flagged. A line scanner with text-heuristic provenance is used only when the file fails to parse.",
        ),
        ..RuleMetadata::DEFAULT
    },
];
