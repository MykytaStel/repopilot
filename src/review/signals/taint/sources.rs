//! Untrusted-input source recognition for taint-lite.
//!
//! AST-node text is checked for the request/argv idioms in the language's
//! [`TaintTables`]. Only member/attribute/selector nodes participate, so
//! examples inside strings or comments do not become sources. Matching is
//! whole-token (via [`super::contains_token`]) so `req.query` is recognized
//! in `req.query.id` but not inside `req.queryString`. The idiom lists are
//! conservative on purpose — only high-signal, widely-used idioms — and env
//! vars are intentionally excluded: they are flagged separately as a
//! behavioral signal and are rarely user-controlled.

use super::contains_token;
use super::tables::TaintTables;
use serde::Serialize;
use tree_sitter::Node;

/// Where an untrusted value originates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    /// A field of an inbound HTTP request (query, params, body, headers, …).
    HttpRequest,
    /// The process command-line arguments.
    ProcessArgs,
}

impl SourceKind {
    /// Human-readable label used in detail text.
    pub fn label(self) -> &'static str {
        match self {
            Self::HttpRequest => "HTTP request input",
            Self::ProcessArgs => "process arguments",
        }
    }
}

/// The untrusted-input source referenced under `node`, if any. Request idioms
/// win over argv when both appear.
pub(super) fn node_has_source(
    node: Node<'_>,
    content: &str,
    tables: &TaintTables,
) -> Option<SourceKind> {
    if node_has_patterns(node, content, tables, tables.request_sources) {
        return Some(SourceKind::HttpRequest);
    }
    if node_has_patterns(node, content, tables, tables.argv_sources) {
        return Some(SourceKind::ProcessArgs);
    }
    None
}

fn node_has_patterns(
    node: Node<'_>,
    content: &str,
    tables: &TaintTables,
    patterns: &[&str],
) -> bool {
    if (tables.is_flow_scope)(node) {
        return false;
    }
    // A sanitizer/coercion call neutralizes whatever it wraps; do not descend.
    if super::sanitizers::is_sanitizer_call(node, content, tables) {
        return false;
    }

    if tables.source_access_kinds.contains(&node.kind()) {
        let text = node.utf8_text(content.as_bytes()).unwrap_or("");
        if patterns.iter().any(|pattern| contains_token(text, pattern)) {
            return true;
        }
    }

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| node_has_patterns(child, content, tables, patterns))
}
