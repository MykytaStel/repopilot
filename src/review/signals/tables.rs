//! Per-language review-signal tables owned by the language frontends.
//!
//! Like taint's [`TaintTables`](super::taint::tables::TaintTables), these
//! carry the grammar node-kind sets the review engines consult, so the
//! engines in [`super::classify`] and [`super::algorithmic`] stay
//! language-neutral. The keyword vocabularies themselves (auth tokens,
//! library names) are cross-language and stay in the engines; only node
//! shapes vary per grammar.

/// Node kinds that can carry a security-boundary name for one language.
pub struct BoundaryKinds {
    /// Decorator/annotation/attribute-like nodes — checked for access-control
    /// vocabulary only (a CORS library cannot appear here).
    pub(crate) decorator_kinds: &'static [&'static str],
    /// Import/call-like nodes — checked for access-control vocabulary *and*
    /// request-trust libraries (CORS / security headers).
    pub(crate) import_kinds: &'static [&'static str],
}

/// Node kinds backing the algorithmic-shift signals for one language.
pub struct AlgorithmicKinds {
    /// Named-function declaration kinds (anonymous functions are skipped).
    pub(crate) function_kinds: &'static [&'static str],
    /// Loop statement/expression kinds.
    pub(crate) loop_kinds: &'static [&'static str],
    /// Call node kinds, for recursion detection.
    pub(crate) call_kinds: &'static [&'static str],
    /// Control-flow kinds that add a nesting level.
    pub(crate) control_flow_kinds: &'static [&'static str],
    /// `if` kinds, for collapsing `else if` chains to one nesting level.
    pub(crate) if_kinds: &'static [&'static str],
}

/// Recognizers for the "removed behavioral signal" detectors (deleted tests,
/// removed error handling, removed auth checks).
pub struct RemovedTables {
    /// Extensions this table answers for. This dispatch predates label-based
    /// detection and is kept verbatim — including `cts`, which detection
    /// never labels today, preserved as-is rather than silently dropped.
    pub(crate) extensions: &'static [&'static str],
    /// Whether a node declares one test case (`it(...)`, `def test_…`, …).
    pub(crate) is_test_case: for<'a> fn(tree_sitter::Node<'a>, &'a str) -> bool,
    /// Whether a node is an error-handling construct (try/catch,
    /// `if err != nil`, `match … Err(…)`).
    pub(crate) is_error_handling: for<'a> fn(tree_sitter::Node<'a>, &'a str) -> bool,
    /// Call node kinds inspected for auth-check callees.
    pub(crate) auth_call_kinds: &'static [&'static str],
}

/// The review-signal tables a frontend registers.
pub struct ReviewTables {
    /// Boundary node kinds; `None` when the language's AST boundary
    /// classification is not wired (C# today — its label never matched the
    /// old dispatch, and enabling it is a behavior change for a later PR).
    pub(crate) boundary: Option<&'static BoundaryKinds>,
    pub(crate) algorithmic: &'static AlgorithmicKinds,
    /// Removed-behavior recognizers; extension-dispatched (legacy).
    pub(crate) removed: Option<&'static RemovedTables>,
}
