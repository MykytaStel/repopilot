//! Coercion recognition for taint-lite.
//!
//! A value that reaches a sink only through a numeric/boolean **type coercion** —
//! `Number(req.query.id)`, `parseInt(...)`, `int(...)`, `strconv.Atoi(...)`, … —
//! cannot carry an injection into *any* sink: the result is a non-string
//! primitive. The source/tainted-name walkers in [`super::sources`] and
//! [`super::flow`] treat such a call as an opaque, clean subtree and do not
//! descend into its arguments. Pruning is subtree-local, so `Number(a) + b` still
//! flags `b`.
//!
//! Only universally-neutralizing coercions are recognized here, deliberately.
//! Context-specific sanitizers (shell quoting like `shlex.quote`, URL encoding
//! like `encodeURIComponent`, HTML escaping like `escape`) only neutralize for
//! *their* sink and would cause false negatives if applied to SQL/exec/fs
//! universally — recognizing them per-sink is a documented follow-up.

use super::TaintLang;
use tree_sitter::Node;

const JS_COERCIONS: &[&str] = &["Number", "parseInt", "parseFloat", "BigInt", "Boolean"];

const PY_COERCIONS: &[&str] = &["int", "float", "bool"];

const GO_COERCIONS: &[&str] = &[
    "strconv.Atoi",
    "strconv.ParseInt",
    "strconv.ParseFloat",
    "strconv.ParseBool",
];

/// Whether `node` is a call to a numeric/boolean coercion that universally
/// neutralizes its argument, so the walkers should not descend into it.
pub(super) fn is_sanitizer_call(node: Node<'_>, content: &str, lang: TaintLang) -> bool {
    let is_call = match lang {
        TaintLang::Js | TaintLang::Go => node.kind() == "call_expression",
        TaintLang::Python => node.kind() == "call",
    };
    if !is_call {
        return false;
    }
    let Some(callee) = node.child_by_field_name("function") else {
        return false;
    };
    let text = callee.utf8_text(content.as_bytes()).unwrap_or("").trim();

    let coercions = match lang {
        TaintLang::Js => JS_COERCIONS,
        TaintLang::Python => PY_COERCIONS,
        TaintLang::Go => GO_COERCIONS,
    };
    coercions.contains(&text)
}
