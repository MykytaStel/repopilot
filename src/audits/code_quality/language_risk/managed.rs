use super::{line_of, node_text, push_pattern_finding, snippet_of};
use crate::findings::types::{Finding, Severity};
use crate::scan::facts::FileFacts;
use std::path::Path;
use tree_sitter::Node;

// ── Java / Kotlin / C# ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) enum ManagedRiskPattern {
    FatalException { is_csharp: bool },
    NotImplemented,
}

impl ManagedRiskPattern {
    pub(super) const JVM_PATTERNS: &'static [Self] = &[
        Self::FatalException { is_csharp: false },
        Self::NotImplemented,
    ];
    pub(super) const CSHARP_PATTERNS: &'static [Self] = &[
        Self::FatalException { is_csharp: true },
        Self::NotImplemented,
    ];

    pub(super) fn matches(self, trimmed: &str, _path: &Path) -> bool {
        match self {
            Self::FatalException { is_csharp } => {
                trimmed.contains("throw new RuntimeException(")
                    || trimmed.contains("throw new IllegalStateException(")
                    || (is_csharp && trimmed.contains("throw new Exception("))
            }
            Self::NotImplemented => {
                trimmed.contains("throw new NotImplementedException(")
                    || trimmed.contains("throw new NotImplementedError(")
                    || trimmed.contains("TODO(")
                    || trimmed.contains("TODO()")
            }
        }
    }

    pub(super) fn rule_id(self) -> &'static str {
        "language.managed.fatal-exception-risk"
    }

    pub(super) fn signal(self) -> &'static str {
        match self {
            Self::FatalException { .. } => "managed.fatal-exception",
            Self::NotImplemented => "managed.not-implemented",
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::FatalException { .. } => "Generic fatal exception in managed code",
            Self::NotImplemented => "Not-implemented placeholder in managed code",
        }
    }

    pub(super) fn context_label(self) -> &'static str {
        match self {
            Self::FatalException { .. } => "JVM/.NET generic fatal exception",
            Self::NotImplemented => "JVM/.NET placeholder failure",
        }
    }

    pub(super) fn recommendation(self) -> &'static str {
        match self {
            Self::FatalException { .. } => {
                "Use domain-specific exception or result types when callers need precise recovery behaviour."
            }
            Self::NotImplemented => {
                "Replace placeholders before production release or isolate unfinished paths clearly."
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

/// Emits Java runtime-risk findings from the syntax tree: generic fatal
/// `throw new RuntimeException/IllegalStateException(...)`, not-implemented
/// throws, and `TODO(...)` placeholders.
pub(super) fn emit_java_node(
    node: Node<'_>,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    let pattern = match node.kind() {
        "throw_statement" => {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .find(|child| child.kind() == "object_creation_expression")
                .and_then(|child| child.child_by_field_name("type"))
                .and_then(|type_node| node_text(type_node, content))
                .and_then(|type_name| match type_name {
                    "RuntimeException" | "IllegalStateException" => {
                        Some(ManagedRiskPattern::FatalException { is_csharp: false })
                    }
                    "NotImplementedException" | "NotImplementedError" => {
                        Some(ManagedRiskPattern::NotImplemented)
                    }
                    _ => None,
                })
        }
        "method_invocation" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if node_text(name_node, content) == Some("TODO") {
                    Some(ManagedRiskPattern::NotImplemented)
                } else {
                    None
                }
            } else {
                None
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

/// Emits Kotlin runtime-risk findings: generic fatal `throw RuntimeException/
/// IllegalStateException(...)`, not-implemented throws, and `TODO(...)`.
pub(super) fn emit_kotlin_node(
    node: Node<'_>,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    let pattern = match node.kind() {
        "throw_expression" => {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .find(|child| child.kind() == "call_expression")
                .and_then(|child| child.child(0))
                .and_then(|callee| node_text(callee, content))
                .and_then(|callee_name| match callee_name {
                    "RuntimeException" | "IllegalStateException" => {
                        Some(ManagedRiskPattern::FatalException { is_csharp: false })
                    }
                    "NotImplementedException" | "NotImplementedError" => {
                        Some(ManagedRiskPattern::NotImplemented)
                    }
                    _ => None,
                })
        }
        "call_expression" => {
            if let Some(callee) = node.child(0) {
                if node_text(callee, content) == Some("TODO") {
                    Some(ManagedRiskPattern::NotImplemented)
                } else {
                    None
                }
            } else {
                None
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

/// Emits C# runtime-risk findings: generic fatal `throw new Exception/
/// RuntimeException/IllegalStateException(...)`, not-implemented throws, and
/// `TODO(...)`.
pub(super) fn emit_csharp_node(
    node: Node<'_>,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    let pattern = match node.kind() {
        "throw_statement" | "throw_expression" => {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .find(|child| child.kind() == "object_creation_expression")
                .and_then(|child| child.child_by_field_name("type"))
                .and_then(|type_node| node_text(type_node, content))
                .and_then(|type_name| match type_name {
                    "Exception" | "RuntimeException" | "IllegalStateException" => {
                        Some(ManagedRiskPattern::FatalException { is_csharp: true })
                    }
                    "NotImplementedException" | "NotImplementedError" => {
                        Some(ManagedRiskPattern::NotImplemented)
                    }
                    _ => None,
                })
        }
        "invocation_expression" => {
            // C# `invocation_expression` exposes the callee under the `function`
            // field; `child(0)` is a defensive fallback for older grammar shapes.
            let name_node = node
                .child_by_field_name("function")
                .or_else(|| node.child(0));
            if let Some(name_node) = name_node {
                if node_text(name_node, content) == Some("TODO") {
                    Some(ManagedRiskPattern::NotImplemented)
                } else {
                    None
                }
            } else {
                None
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
