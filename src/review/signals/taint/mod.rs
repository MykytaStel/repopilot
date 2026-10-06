//! Taint-lite reachability signals for `repopilot review`.
//!
//! Within each changed file's post-change source, recognize selected untrusted
//! inputs (such as request fields or process arguments) reaching SQL, process,
//! filesystem-write, or network sinks. Within one flow scope, it follows local
//! assignments and selected static property/index paths. It does not follow
//! values across functions, dynamic indexes, or general heap aliases.
//!
//! For SQL, the tainted value must be part of the query string. A value passed
//! as a separate bind argument is not reported. Test files and languages without
//! a configured taint frontend are skipped. A signal records a recognized
//! source-to-sink path; it does not establish exploitability.
//!
//! - [`sources`] recognizes untrusted-input access nodes per language.
//! - [`sinks`] classifies a call node as a dangerous sink and exposes its args.
//! - [`flow`] tracks local assignments and emits sink lines that overlap the
//!   changed ranges.

pub(crate) mod ast;
#[cfg(test)]
mod csharp_property_tests;
mod flow;
mod sanitizers;
pub(crate) mod sinks;
mod sources;
pub(crate) mod tables;
#[cfg(test)]
mod tests;

use crate::review::diff::ChangedFile;
use crate::review::signals::content::ReviewSource;
use crate::scan::language::detect_language;
use serde::Serialize;

pub use sinks::SinkKind;
pub use sources::SourceKind;

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
    if crate::audits::context::classify::helpers::is_test_file(&file.path) {
        return Vec::new();
    }
    let Some(post) = post_source else {
        return Vec::new();
    };
    let Some(tables) = detect_language(&file.path).and_then(crate::languages::taint_for_label)
    else {
        return Vec::new();
    };

    let Some(tree) = post.tree() else {
        return Vec::new();
    };

    let mut signals = Vec::new();
    flow::detect(tree.root_node(), post.content(), tables, file, &mut signals);

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
