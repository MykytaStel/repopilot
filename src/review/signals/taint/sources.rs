//! Untrusted-input source recognition for taint-lite.
//!
//! AST-node text is checked for known request/argv idioms. Only member/attribute/
//! selector nodes participate, so examples inside strings or comments do not
//! become sources. Matching is whole-token (via [`super::contains_token`]) so
//! `req.query` is recognized in `req.query.id` but not inside `req.queryString`.
//! Conservative on purpose: only high-signal, widely-used idioms across
//! Express/Koa (JS), Flask/Django/FastAPI (Python), and net/http/Gin (Go). Env
//! vars are intentionally excluded here: they are flagged separately as a
//! behavioral signal and are rarely user-controlled.

use super::{TaintLang, contains_token};
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

const JS_REQUEST: &[&str] = &[
    "req.query",
    "req.params",
    "req.body",
    "req.headers",
    "req.cookies",
    "req.url",
    "request.query",
    "request.params",
    "request.body",
    "request.headers",
    "request.cookies",
    "ctx.query",
    "ctx.request.body",
];

const PY_REQUEST: &[&str] = &[
    "request.args",
    "request.form",
    "request.json",
    "request.values",
    "request.data",
    "request.files",
    "request.GET",
    "request.POST",
    "request.query_params",
    "request.path_params",
    "request.body",
    "request.cookies",
    "request.headers",
];

const GO_REQUEST: &[&str] = &[
    "r.URL.Query",
    "r.FormValue",
    "r.PostFormValue",
    "r.PostForm",
    "r.Form",
    "r.Body",
    "r.Header.Get",
    "c.Query",
    "c.Param",
    "c.PostForm",
    "ctx.Query",
];

/// The untrusted-input source referenced under `node`, if any. Request idioms
/// win over argv when both appear.
pub(super) fn node_has_source(
    node: Node<'_>,
    content: &str,
    lang: TaintLang,
) -> Option<SourceKind> {
    let (request, argv): (&[&str], &[&str]) = match lang {
        TaintLang::Js => (JS_REQUEST, &["process.argv"]),
        TaintLang::Python => (PY_REQUEST, &["sys.argv"]),
        TaintLang::Go => (GO_REQUEST, &["os.Args"]),
    };
    if node_has_patterns(node, content, lang, request) {
        return Some(SourceKind::HttpRequest);
    }
    if node_has_patterns(node, content, lang, argv) {
        return Some(SourceKind::ProcessArgs);
    }
    None
}

fn node_has_patterns(node: Node<'_>, content: &str, lang: TaintLang, patterns: &[&str]) -> bool {
    if lang.is_flow_scope(node) {
        return false;
    }

    let is_access = match lang {
        TaintLang::Js => node.kind() == "member_expression",
        TaintLang::Python => node.kind() == "attribute",
        TaintLang::Go => node.kind() == "selector_expression",
    };
    if is_access {
        let text = node.utf8_text(content.as_bytes()).unwrap_or("");
        if patterns.iter().any(|pattern| contains_token(text, pattern)) {
            return true;
        }
    }

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| node_has_patterns(child, content, lang, patterns))
}
