//! Dangerous-sink model for taint-lite.
//!
//! The per-language sink classifiers live on the language frontends
//! (`languages/*/review.rs`, wired through
//! [`TaintTables::classify_sink`](super::tables::TaintTables)); this module
//! keeps the shared model — [`SinkKind`], [`Sink`] — and the callee-shape
//! helpers the classifiers share. Classification stays text/callee-name based
//! to mirror the behavioral "added X" detectors and stay robust across
//! grammars.

use serde::Serialize;
use tree_sitter::Node;

/// The kind of dangerous operation a sink performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SinkKind {
    /// A raw SQL query/exec (`.query`, `.execute`, `db.Exec`, …).
    Sql,
    /// A subprocess / shell execution (`child_process.exec`, `subprocess.run`, …).
    Exec,
    /// A filesystem write or path-controlled open (`fs.writeFile`, `open(.., 'w')`).
    FsWrite,
    /// An outbound network call (`fetch`, `requests.get`, `http.Get`, …).
    Network,
}

impl SinkKind {
    /// Human-readable label used in detail/headline text.
    pub fn label(self) -> &'static str {
        match self {
            Self::Sql => "raw SQL query",
            Self::Exec => "subprocess/exec",
            Self::FsWrite => "filesystem write",
            Self::Network => "network call",
        }
    }
}

/// A classified sink call: its kind and the AST node holding its arguments.
pub(crate) struct Sink<'a> {
    pub kind: SinkKind,
    pub args: Node<'a>,
}

/// The callee text and argument node of a call node, if it is one.
pub(crate) fn callee_text<'a>(node: Node<'a>, content: &'a str) -> Option<(&'a str, Node<'a>)> {
    if node.kind() != "call_expression" && node.kind() != "call" {
        return None;
    }
    let callee = node.child_by_field_name("function")?;
    let args = node.child_by_field_name("arguments")?;
    let text = callee.utf8_text(content.as_bytes()).ok()?.trim();
    Some((text, args))
}

pub(crate) fn receiver_method(callee: &str, receiver: &str, methods: &[&str]) -> bool {
    methods
        .iter()
        .any(|method| callee == format!("{receiver}.{method}"))
}
