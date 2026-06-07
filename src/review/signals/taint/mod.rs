//! Taint-lite reachability signals for `repopilot review`.
//!
//! Intra-procedural by design: within a changed file's post-change source, flag
//! when a value derived from an untrusted *source* (an HTTP request field, the
//! process arguments) reaches a dangerous *sink* (a raw SQL query, a
//! subprocess/exec call, a filesystem write, an outbound network call). It tracks
//! direct flow and simple local assignment chains (`x = req.query.id; run(x)`),
//! and stops there — no cross-function, aliasing, or whole-program analysis. That
//! is the "lite" in taint-lite.
//!
//! Consistent with the "flag, don't prove" stance in the parent `mod.rs`: a
//! signal says a *path exists* from input to a sink, never that it is
//! exploitable. For SQL we only flag when the tainted value is built *into* the
//! query string (concatenation / interpolation / format); a parameterized query
//! that passes the value as a separate bind argument is the safe pattern and is
//! deliberately not flagged. Test files are skipped. Ships at `preview`.
//!
//! - [`sources`] recognizes untrusted-input access nodes per language.
//! - [`sinks`] classifies a call node as a dangerous sink and exposes its args.
//! - [`flow`] seeds tainted locals from sources and reports when one reaches a
//!   sink, gated to the changed lines so it stays change-scoped.

mod ast;
mod flow;
mod sanitizers;
mod sinks;
mod sources;
#[cfg(test)]
mod tests;

use crate::review::diff::ChangedFile;
use crate::review::signals::content::ReviewSource;
use crate::scan::language::detect_language;
use serde::Serialize;
use tree_sitter::Node;

pub use sinks::SinkKind;
pub use sources::SourceKind;

/// The languages taint-lite understands. JS/TS (and their React variants) share a
/// grammar shape, so one matcher covers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaintLang {
    Js,
    Python,
    Go,
}

impl TaintLang {
    fn from_label(label: &str) -> Option<Self> {
        match label {
            "JavaScript" | "JavaScript React" | "TypeScript" | "TypeScript React" => Some(Self::Js),
            "Python" => Some(Self::Python),
            "Go" => Some(Self::Go),
            _ => None,
        }
    }

    fn is_flow_scope(self, node: Node<'_>) -> bool {
        match self {
            Self::Js => matches!(
                node.kind(),
                "function_declaration"
                    | "generator_function_declaration"
                    | "method_definition"
                    | "function_expression"
                    | "generator_function"
                    | "arrow_function"
            ),
            Self::Python => matches!(node.kind(), "function_definition" | "lambda"),
            Self::Go => matches!(
                node.kind(),
                "function_declaration" | "method_declaration" | "func_literal"
            ),
        }
    }
}

/// One untrusted-input-reaches-sink flow found in a changed file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaintSignal {
    pub source: SourceKind,
    pub sink: SinkKind,
    pub path: String,
    /// Line of the sink call — the dangerous site — 1-based.
    pub line: usize,
    /// Neutral structural fact naming the source idiom and the sink call.
    pub detail: String,
}

/// Detect taint-lite flows in a changed file's post-change source.
///
/// Returns at most one signal per (sink line, sink kind) so nested AST matches do
/// not double-report. Skips test files and any language without a grammar here.
pub fn detect_taint(file: &ChangedFile, post_source: Option<&ReviewSource>) -> Vec<TaintSignal> {
    if crate::audits::context::classify::helpers::is_test_file(&file.path, false) {
        return Vec::new();
    }
    let Some(post) = post_source else {
        return Vec::new();
    };
    let Some(lang) = detect_language(&file.path).and_then(TaintLang::from_label) else {
        return Vec::new();
    };

    let parsed = post.parsed();
    let Some(tree) = parsed.tree() else {
        return Vec::new();
    };

    let mut signals = Vec::new();
    flow::detect(tree.root_node(), post.content(), lang, file, &mut signals);

    let mut unique: Vec<TaintSignal> = Vec::new();
    for signal in signals {
        if !unique
            .iter()
            .any(|existing| existing.line == signal.line && existing.sink == signal.sink)
        {
            unique.push(signal);
        }
    }
    unique
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `needle` occurs in `haystack` bounded by non-identifier characters on
/// both ends, so a name/idiom is matched as a whole token. `req.query` matches in
/// `req.query.id` (next char `.`) but not in `req.queryString` (next char `S`),
/// and a tainted local `id` matches in `run(id)` but not in `run(valid)`.
pub(super) fn contains_token(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let mut start = 0;
    while let Some(rel) = haystack[start..].find(needle) {
        let s = start + rel;
        let e = s + needle.len();
        let before_ok = s == 0 || !haystack[..s].chars().next_back().is_some_and(is_ident_char);
        let after_ok =
            e >= haystack.len() || !haystack[e..].chars().next().is_some_and(is_ident_char);
        if before_ok && after_ok {
            return true;
        }
        start = e;
    }
    false
}
