use super::{line_of, node_text, push_pattern_finding, snippet_of};
use crate::findings::types::{Finding, Severity};
use crate::scan::facts::FileFacts;
use std::path::Path;
use tree_sitter::Node;

// ── JavaScript / TypeScript ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) enum JsRiskPattern {
    ProcessExit,
}

impl JsRiskPattern {
    pub(super) const ALL: &'static [Self] = &[Self::ProcessExit];

    pub(super) fn matches(self, trimmed: &str, _path: &Path) -> bool {
        match self {
            // Severity is decided downstream by the knowledge engine: a
            // `process.exit` in a CLI package, `bin/`/`scripts/` path, or
            // app-entrypoint is downgraded to Low (hidden by default, kept in
            // strict), while ordinary reusable code stays High. The detector
            // only reports the call; it never suppresses outright.
            Self::ProcessExit => trimmed.contains("process.exit("),
        }
    }

    pub(super) fn rule_id(self) -> &'static str {
        "language.javascript.runtime-exit-risk"
    }

    pub(super) fn signal(self) -> &'static str {
        match self {
            Self::ProcessExit => "js.process-exit",
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::ProcessExit => "JavaScript process.exit usage",
        }
    }

    pub(super) fn context_label(self) -> &'static str {
        match self {
            Self::ProcessExit => "JavaScript process exit call",
        }
    }

    pub(super) fn recommendation(self) -> &'static str {
        match self {
            Self::ProcessExit => {
                "Keep process exits at a CLI boundary and return errors from reusable modules."
            }
        }
    }

    pub(super) fn base_severity(self) -> Severity {
        match self {
            Self::ProcessExit => Severity::High,
        }
    }
}

/// Emits JavaScript/TypeScript runtime-risk findings from the syntax tree:
/// `process.exit(...)` calls, which terminate the host process and so are unsafe
/// in reusable library code. A `throw` is recoverable control flow, not a
/// runtime exit, so generic thrown errors are intentionally not flagged here.
///
/// The call is always reported; the knowledge engine then decides severity from
/// file context — a `process.exit` in a CLI package (`package.json#bin`), a
/// `bin/`/`scripts/` path, or an app-entrypoint is downgraded to Low (hidden in
/// the default profile, retained under `--profile strict`), while reusable code
/// keeps it at High.
pub(crate) fn emit_js_node(
    node: Node<'_>,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    let pattern = match node.kind() {
        "call_expression" if is_process_exit_call(node, content) => {
            Some(JsRiskPattern::ProcessExit)
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

/// `process.exit(...)` — a call whose callee is the `process.exit` member.
fn is_process_exit_call(node: Node<'_>, content: &str) -> bool {
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    if function.kind() != "member_expression" {
        return false;
    }
    let object = function
        .child_by_field_name("object")
        .and_then(|n| node_text(n, content));
    let property = function
        .child_by_field_name("property")
        .and_then(|n| node_text(n, content));
    object == Some("process") && property == Some("exit")
}

/// Line-scanner fallback: run every JS-family pattern against one sanitized line.
pub(crate) fn emit_line(
    trimmed: &str,
    path: &Path,
    raw_line: &str,
    line_index: usize,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    for pattern in JsRiskPattern::ALL {
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
