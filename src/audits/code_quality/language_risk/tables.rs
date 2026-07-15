//! The per-language runtime-risk contract.
//!
//! A language frontend that participates in the runtime-risk audit registers
//! a [`RiskTables`]: an AST node emitter, a sanitized-line emitter for the
//! no-parse fallback, and which comment sanitizer its line scanner needs.
//! The audit in [`super`] is language-neutral and dispatches through the
//! frontend registry; the pattern definitions stay in
//! [`super::pattern`]'s per-language modules.

use crate::findings::types::Finding;
use crate::scan::facts::FileFacts;
use std::path::Path;
use tree_sitter::{Node, Tree};

/// Emits findings for one AST node (the walker recurses).
pub(crate) type NodeEmitter = fn(Node<'_>, &str, &Path, &FileFacts, &mut Vec<Finding>);

/// Emits findings for one sanitized, trimmed line.
pub(crate) type LineEmitter = fn(&str, &Path, &str, usize, &FileFacts, &mut Vec<Finding>);

/// Which comment sanitizer the line-scanner fallback needs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RiskLineSanitizer {
    CStyle,
    Python,
}

pub(crate) struct RiskTables {
    pub(crate) emit_node: NodeEmitter,
    pub(crate) emit_line: LineEmitter,
    pub(crate) sanitizer: RiskLineSanitizer,
}

/// Runs `emit` over every node of `tree`, depth-first — the shared walk the
/// per-language node emitters plug into.
pub(crate) fn walk_tree_with(
    emit: NodeEmitter,
    tree: &Tree,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    walk(tree.root_node(), emit, content, path, file, findings);
}

fn walk(
    node: Node<'_>,
    emit: NodeEmitter,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    emit(node, content, path, file, findings);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, emit, content, path, file, findings);
    }
}
