use super::{line_of, node_text, push_pattern_finding, snippet_of};
use crate::findings::types::{Finding, Severity};
use crate::scan::facts::FileFacts;
use std::path::Path;
use tree_sitter::Node;

// ── Python ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) enum PythonRiskPattern {
    BroadExcept,
    Assert,
    NotImplemented,
}

impl PythonRiskPattern {
    pub(super) const ALL: &'static [Self] =
        &[Self::BroadExcept, Self::Assert, Self::NotImplemented];

    pub(super) fn matches(self, trimmed: &str, _path: &Path) -> bool {
        match self {
            Self::BroadExcept => {
                let normalized = trimmed.replace(' ', "");
                normalized == "except:" || normalized.starts_with("except:#")
            }
            Self::Assert => trimmed.starts_with("assert ") || trimmed.starts_with("assert("),
            Self::NotImplemented => {
                trimmed.contains("raise NotImplementedError")
                    || trimmed.contains("NotImplementedError(")
                    || trimmed == "raise NotImplementedError"
            }
        }
    }

    pub(super) fn rule_id(self) -> &'static str {
        "language.python.exception-risk"
    }

    pub(super) fn signal(self) -> &'static str {
        match self {
            Self::BroadExcept => "python.broad-except",
            Self::Assert => "python.assert",
            Self::NotImplemented => "python.not-implemented",
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::BroadExcept => "Broad Python except handler",
            Self::Assert => "Python assert in production path",
            Self::NotImplemented => "Python NotImplementedError placeholder",
        }
    }

    pub(super) fn context_label(self) -> &'static str {
        match self {
            Self::BroadExcept => "Python broad exception handler",
            Self::Assert => "Python assert statement",
            Self::NotImplemented => "Python not-implemented placeholder",
        }
    }

    pub(super) fn recommendation(self) -> &'static str {
        match self {
            Self::BroadExcept => "Catch specific exceptions so unrelated failures are not hidden.",
            Self::Assert => {
                "Use explicit runtime validation for production invariants because asserts can be disabled."
            }
            Self::NotImplemented => {
                "Replace placeholders before production release or guard them behind explicit feature flags."
            }
        }
    }

    pub(super) fn base_severity(self) -> Severity {
        match self {
            Self::NotImplemented => Severity::High,
            _ => Severity::Medium,
        }
    }
}

/// Emits Python runtime-risk findings from the syntax tree: bare `except:`,
/// `assert` statements, and `NotImplementedError` placeholders.
pub(crate) fn emit_python_node(
    node: Node<'_>,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    let pattern = match node.kind() {
        "except_clause" if is_bare_except(node) => Some(PythonRiskPattern::BroadExcept),
        "assert_statement" => Some(PythonRiskPattern::Assert),
        "call" if is_not_implemented_call(node, content) => Some(PythonRiskPattern::NotImplemented),
        "raise_statement" if raises_bare_not_implemented(node, content) => {
            Some(PythonRiskPattern::NotImplemented)
        }
        _ => None,
    };
    if let Some(pattern) = pattern {
        push_pattern_finding(
            &pattern,
            path,
            line_of(node),
            &snippet_of(node, content),
            file,
            findings,
        );
    }
}

/// A bare `except:` clause — an `except_clause` with no exception type before
/// its body block.
fn is_bare_except(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    !node
        .named_children(&mut cursor)
        .any(|child| child.kind() != "block" && child.kind() != "comment")
}

/// A call to `NotImplementedError(...)`.
fn is_not_implemented_call(node: Node<'_>, content: &str) -> bool {
    node.child_by_field_name("function")
        .and_then(|function| node_text(function, content))
        .map(|text| text == "NotImplementedError" || text.ends_with(".NotImplementedError"))
        .unwrap_or(false)
}

/// `raise NotImplementedError` with no call — the raised expression is the bare
/// `NotImplementedError` identifier (the call form is handled separately so a
/// single `raise NotImplementedError(...)` is not counted twice).
fn raises_bare_not_implemented(node: Node<'_>, content: &str) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).any(|child| {
        child.kind() == "identifier" && node_text(child, content) == Some("NotImplementedError")
    })
}

/// Line-scanner fallback: run every Python pattern against one sanitized line.
pub(crate) fn emit_line(
    trimmed: &str,
    path: &Path,
    raw_line: &str,
    line_index: usize,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    for pattern in PythonRiskPattern::ALL {
        if pattern.matches(trimmed, path) {
            push_pattern_finding(
                pattern,
                path,
                line_index + 1,
                raw_line.trim(),
                file,
                findings,
            );
        }
    }
}
