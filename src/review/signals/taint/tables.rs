//! The per-language taint contract.
//!
//! Everything language-specific in taint-lite lives in one table owned by the
//! language's frontend (`languages/*/review.rs`): the untrusted-source idiom
//! lists, the coercion (sanitizer) lists, the grammar node shapes for scopes/
//! assignments/access, and the sink classifier. The engines in [`super::flow`],
//! [`super::sources`], and [`super::sanitizers`] are language-neutral and only
//! consult the table they are handed, so adding taint support for a language
//! means writing a table — not touching the engine.

use super::sinks::Sink;
use tree_sitter::Node;

pub struct TaintTables {
    /// Whole-token idioms that read inbound HTTP request data.
    pub(crate) request_sources: &'static [&'static str],
    /// Whole-token idioms that read process command-line arguments.
    pub(crate) argv_sources: &'static [&'static str],
    /// Node kinds that access a member/attribute/selector — the only nodes
    /// whose text is matched against the source idioms.
    pub(crate) source_access_kinds: &'static [&'static str],
    /// Callees whose result is a non-string primitive; the walkers treat the
    /// call as a clean subtree and do not descend.
    pub(crate) coercions: &'static [&'static str],
    /// The grammar's call node kind, for recognizing coercion calls.
    pub(crate) coercion_call_kind: &'static str,
    /// Node kinds that bind or reassign local names.
    pub(crate) assignment_kinds: &'static [&'static str],
    /// Whether a node opens a new flow scope (function/lambda/closure).
    pub(crate) is_flow_scope: fn(Node<'_>) -> bool,
    /// Whether an assignment combines with the old value (`x += …`) rather
    /// than replacing it — a compound assignment never clears taint.
    pub(crate) is_augmenting: fn(Node<'_>) -> bool,
    /// Whether a node builds a string from parts (concatenation,
    /// interpolation, `format`/`Sprintf`) — where injected input is dangerous.
    pub(crate) is_string_building: for<'a> fn(Node<'a>, &'a str) -> bool,
    /// Classify a call node as a dangerous sink, exposing its argument node.
    pub(crate) classify_sink: for<'a> fn(Node<'a>, &'a str) -> Option<Sink<'a>>,
}
