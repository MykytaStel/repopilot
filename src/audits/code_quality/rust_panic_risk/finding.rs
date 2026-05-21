use super::RULE_ID;
use super::pattern::RustPanicPattern;
use crate::audits::context::{AuditContext, FileRole, RuntimeKind};
use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use crate::scan::facts::FileFacts;

pub(super) fn build_finding(
    file: &FileFacts,
    line_number: usize,
    snippet: &str,
    pattern: RustPanicPattern,
    context: &AuditContext,
    severity: Severity,
) -> Finding {
    let context_label = context_label(context);
    let recommendation = recommendation_for(context, pattern);
    let confidence = confidence_for(context, pattern, severity);
    let confidence_reason = confidence_reason_for(context, pattern);

    Finding {
        id: String::new(),
        rule_id: RULE_ID.to_string(),
        recommendation: recommendation.to_string(),
        title: format!("Risky Rust {} usage in {}", pattern.label(), context_label),
        description: format!(
            "Rust `{}` was found in {}; confidence is {} because {}. Unhandled panic paths can terminate execution or hide recoverable errors in production code.",
            pattern.label(),
            context_label,
            confidence.label(),
            confidence_reason,
        ),
        category: FindingCategory::CodeQuality,
        severity,
        confidence,
        evidence: vec![Evidence {
            path: file.path.clone(),
            line_start: line_number,
            line_end: None,
            snippet: snippet.to_string(),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

fn context_label(context: &AuditContext) -> &'static str {
    if context.is_test {
        return "Rust test code";
    }

    if context.has_runtime(RuntimeKind::RustCli) {
        return "Rust CLI boundary code";
    }

    if context.has_role(FileRole::Domain) {
        return "Rust domain code";
    }

    if context.has_runtime(RuntimeKind::RustLibrary) {
        return "Rust library code";
    }

    "Rust production code"
}

fn confidence_for(
    context: &AuditContext,
    pattern: RustPanicPattern,
    severity: Severity,
) -> Confidence {
    if context.is_test {
        return Confidence::Low;
    }

    if matches!(
        pattern,
        RustPanicPattern::Todo | RustPanicPattern::Unimplemented
    ) {
        return Confidence::High;
    }

    if context.has_runtime(RuntimeKind::RustCli) {
        return match pattern {
            RustPanicPattern::Unwrap | RustPanicPattern::Expect => Confidence::Low,
            RustPanicPattern::Panic => Confidence::Medium,
            RustPanicPattern::Todo | RustPanicPattern::Unimplemented => Confidence::High,
        };
    }

    if context.has_role(FileRole::Domain) || context.has_runtime(RuntimeKind::RustLibrary) {
        return Confidence::High;
    }

    match severity {
        Severity::High | Severity::Critical => Confidence::High,
        Severity::Info | Severity::Low => Confidence::Low,
        Severity::Medium => Confidence::Medium,
    }
}

fn confidence_reason_for(context: &AuditContext, pattern: RustPanicPattern) -> &'static str {
    if context.is_test {
        return "test code often uses panic-style macros for assertion setup or unfinished test scaffolding";
    }

    if matches!(
        pattern,
        RustPanicPattern::Todo | RustPanicPattern::Unimplemented
    ) {
        return "placeholder macros always panic if this path is reached";
    }

    if context.has_runtime(RuntimeKind::RustCli) {
        return match pattern {
            RustPanicPattern::Unwrap | RustPanicPattern::Expect => {
                "CLI boundary code may intentionally fail fast, but user-facing errors are usually better"
            }
            RustPanicPattern::Panic => {
                "CLI boundary code can terminate the process intentionally, but panic output is rarely a good user-facing error"
            }
            RustPanicPattern::Todo | RustPanicPattern::Unimplemented => {
                "placeholder macros always panic if this path is reached"
            }
        };
    }

    if context.has_role(FileRole::Domain) {
        return "domain code is usually reusable production logic, so callers cannot recover from this panic path";
    }

    if context.has_runtime(RuntimeKind::RustLibrary) {
        return "library code is reused by callers, so panics become part of its public failure behavior";
    }

    "this is production Rust code and the panic path is not locally handled"
}

fn recommendation_for(context: &AuditContext, pattern: RustPanicPattern) -> &'static str {
    if context.is_test {
        return "Panic-style helpers in tests can be acceptable, but keep them out of reusable test utilities when possible.";
    }

    match pattern {
        RustPanicPattern::Unwrap => {
            if context.has_runtime(RuntimeKind::RustCli) {
                "At CLI boundaries this may be acceptable for prototype code, but prefer returning a user-friendly error with context."
            } else {
                "Return `Result` or propagate with `?`; convert to `expect()` only when failure is impossible and the message documents the invariant."
            }
        }
        RustPanicPattern::Expect => {
            if context.has_runtime(RuntimeKind::RustCli) {
                "At CLI boundaries this may be acceptable for prototype code, but prefer returning a user-friendly error with context."
            } else {
                "Prefer `Result`/`?` for recoverable errors. Keep `expect()` only for impossible states, with a message that names the invariant."
            }
        }
        RustPanicPattern::Panic => {
            "Avoid panics in reusable production code. Prefer typed errors, validation, or explicit fallback behavior."
        }
        RustPanicPattern::Todo | RustPanicPattern::Unimplemented => {
            "Replace placeholder macros before release or guard them behind test-only code paths."
        }
    }
}
