use super::{is_library_boundary_path, line_of, node_text, push_pattern_finding, snippet_of};
use crate::findings::types::{Finding, Severity};
use crate::scan::facts::FileFacts;
use std::path::Path;
use tree_sitter::Node;

// ── JavaScript / TypeScript ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) enum JsRiskPattern {
    ProcessExit,
    ThrowError,
}

impl JsRiskPattern {
    pub(super) const ALL: &'static [Self] = &[Self::ProcessExit, Self::ThrowError];

    pub(super) fn matches(self, trimmed: &str, path: &Path) -> bool {
        match self {
            Self::ProcessExit => trimmed.contains("process.exit("),
            Self::ThrowError => {
                trimmed.contains("throw new Error(") && is_library_boundary_path(path)
            }
        }
    }

    pub(super) fn rule_id(self) -> &'static str {
        "language.javascript.runtime-exit-risk"
    }

    pub(super) fn signal(self) -> &'static str {
        match self {
            Self::ProcessExit => "js.process-exit",
            Self::ThrowError => "js.throw-error",
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::ProcessExit => "JavaScript process.exit usage",
            Self::ThrowError => "Generic JavaScript error at library boundary",
        }
    }

    pub(super) fn context_label(self) -> &'static str {
        match self {
            Self::ProcessExit => "JavaScript process exit call",
            Self::ThrowError => "JavaScript generic thrown error",
        }
    }

    pub(super) fn recommendation(self) -> &'static str {
        match self {
            Self::ProcessExit => {
                "Keep process exits at a CLI boundary and return errors from reusable modules."
            }
            Self::ThrowError => {
                "Prefer typed errors or actionable error messages at reusable package boundaries."
            }
        }
    }

    pub(super) fn base_severity(self) -> Severity {
        match self {
            Self::ProcessExit => Severity::High,
            Self::ThrowError => Severity::Medium,
        }
    }
}

/// Emits JavaScript/TypeScript runtime-risk findings from the syntax tree:
/// `process.exit(...)` calls and `throw new Error(...)` at a library boundary.
pub(super) fn emit_js_node(
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
        "throw_statement" if throws_new_error(node, content) && is_library_boundary_path(path) => {
            Some(JsRiskPattern::ThrowError)
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

/// `throw new Error(...)` — a throw whose direct expression constructs `Error`.
fn throws_new_error(node: Node<'_>, content: &str) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| is_new_error(child, content))
}

fn is_new_error(node: Node<'_>, content: &str) -> bool {
    node.kind() == "new_expression"
        && node
            .child_by_field_name("constructor")
            .and_then(|c| node_text(c, content))
            == Some("Error")
}
