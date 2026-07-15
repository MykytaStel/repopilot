use super::{line_of, node_text, push_pattern_finding, snippet_of};
use crate::findings::types::{Finding, Severity};
use crate::scan::facts::FileFacts;
use std::path::Path;
use tree_sitter::Node;

// ── Go ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) enum GoRiskPattern {
    Panic,
    LogFatal,
    OsExit,
}

impl GoRiskPattern {
    pub(super) const ALL: &'static [Self] = &[Self::Panic, Self::LogFatal, Self::OsExit];

    pub(super) fn matches(self, trimmed: &str, _path: &Path) -> bool {
        match self {
            Self::Panic => trimmed.contains("panic("),
            Self::LogFatal => trimmed.contains("log.Fatal(") || trimmed.contains("log.Fatalf("),
            Self::OsExit => trimmed.contains("os.Exit("),
        }
    }

    pub(super) fn rule_id(self) -> &'static str {
        "language.go.panic-exit-risk"
    }

    pub(super) fn signal(self) -> &'static str {
        match self {
            Self::Panic => "go.panic",
            Self::LogFatal => "go.log-fatal",
            Self::OsExit => "go.os-exit",
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Panic => "Risky Go panic usage",
            Self::LogFatal => "Risky Go log.Fatal usage",
            Self::OsExit => "Risky Go os.Exit usage",
        }
    }

    pub(super) fn context_label(self) -> &'static str {
        match self {
            Self::Panic => "Go panic call",
            Self::LogFatal => "Go fatal logging call",
            Self::OsExit => "Go process exit call",
        }
    }

    pub(super) fn recommendation(self) -> &'static str {
        match self {
            Self::Panic => {
                "Return an error from reusable Go code and let the caller decide how to recover."
            }
            Self::LogFatal => {
                "Use returned errors outside the narrow CLI boundary so libraries remain recoverable."
            }
            Self::OsExit => {
                "Centralise process exits at the CLI entrypoint and return errors elsewhere."
            }
        }
    }

    pub(super) fn base_severity(self) -> Severity {
        Severity::Medium
    }
}

/// Emits Go runtime-risk findings from the syntax tree: `panic(...)`,
/// `log.Fatal`/`log.Fatalf`, and `os.Exit`.
pub(crate) fn emit_go_node(
    node: Node<'_>,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    if node.kind() != "call_expression" {
        return;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };

    let pattern = match function.kind() {
        // `panic(...)`
        "identifier" if node_text(function, content) == Some("panic") => Some(GoRiskPattern::Panic),
        // `pkg.Fn(...)` — `log.Fatal`/`log.Fatalf` and `os.Exit`.
        "selector_expression" => {
            let package = function
                .child_by_field_name("operand")
                .and_then(|n| node_text(n, content));
            let method = function
                .child_by_field_name("field")
                .and_then(|n| node_text(n, content));
            match (package, method) {
                (Some("log"), Some("Fatal" | "Fatalf")) => Some(GoRiskPattern::LogFatal),
                (Some("os"), Some("Exit")) => Some(GoRiskPattern::OsExit),
                _ => None,
            }
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

/// Line-scanner fallback: run every Go pattern against one sanitized line.
pub(crate) fn emit_line(
    trimmed: &str,
    path: &Path,
    raw_line: &str,
    line_index: usize,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    for pattern in GoRiskPattern::ALL {
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
