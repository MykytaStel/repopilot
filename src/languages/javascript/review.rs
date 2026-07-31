//! Taint tables for the JS dialect family. JS/TS (and their React variants)
//! share a grammar shape, so one table covers both frontends. Source idioms
//! target Express/Koa, Next.js App Router, Fastify, and Hono; sinks mirror the
//! behavioral "added X" detectors.

use crate::review::signals::tables::{
    AlgorithmicKinds, BoundaryKinds, RemovedTables, ReviewTables,
};
use crate::review::signals::taint::sinks::{Sink, SinkKind, callee_text, receiver_method};
use crate::review::signals::taint::tables::TaintTables;
use tree_sitter::Node;

pub(super) static JS_FAMILY_TAINT: TaintTables = TaintTables {
    request_sources: &[
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
        "request.url",
        "request.nextUrl.searchParams",
        "req.nextUrl.searchParams",
        "request.json",
        "req.json",
        "request.formData",
        "req.formData",
        "request.raw.url",
        "req.raw.url",
        "request.ip",
        "req.ip",
        "request.ips",
        "req.ips",
        "request.hostname",
        "req.hostname",
        "request.protocol",
        "req.protocol",
        "ctx.query",
        "ctx.request.body",
        "c.req.query",
        "c.req.queries",
        "c.req.param",
        "c.req.header",
        "c.req.json",
        "c.req.parseBody",
        "c.req.text",
        "c.req.arrayBuffer",
        "c.req.raw",
        "c.req.url",
    ],
    argv_sources: &["process.argv"],
    source_access_kinds: &["member_expression"],
    coercions: &["Number", "parseInt", "parseFloat", "BigInt", "Boolean"],
    coercion_call_kind: "call_expression",
    assignment_kinds: &["variable_declarator", "assignment_expression"],
    is_flow_scope,
    // tree-sitter-javascript models `x += …` as a distinct
    // `augmented_assignment_expression` node that assignment collection does
    // not pick up, so anything collected is a plain `=`.
    is_augmenting: |_| false,
    is_string_building,
    classify_sink,
};

fn is_flow_scope(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "function_declaration"
            | "generator_function_declaration"
            | "method_definition"
            | "function_expression"
            | "generator_function"
            | "arrow_function"
    )
}

fn is_string_building(node: Node<'_>, _content: &str) -> bool {
    matches!(node.kind(), "template_string" | "binary_expression")
}

fn classify_sink<'a>(node: Node<'a>, content: &'a str) -> Option<Sink<'a>> {
    let (callee, args) = callee_text(node, content)?;
    let kind = if callee.ends_with(".query")
        || callee == "query"
        || callee.ends_with(".execute")
        || callee == "execute"
    {
        SinkKind::Sql
    } else if callee_is_or_ends_with(
        callee,
        &[
            "exec",
            "spawn",
            "execFile",
            "fork",
            "execSync",
            "spawnSync",
            "execFileSync",
        ],
    ) {
        SinkKind::Exec
    } else if callee_is_or_ends_with(
        callee,
        &[
            "writeFile",
            "writeFileSync",
            "appendFile",
            "appendFileSync",
            "createWriteStream",
        ],
    ) {
        SinkKind::FsWrite
    } else if callee == "fetch"
        || callee == "axios"
        || receiver_method(
            callee,
            "axios",
            &[
                "request", "get", "post", "put", "patch", "delete", "head", "options",
            ],
        )
        || matches!(
            callee,
            "http.request" | "https.request" | "http.get" | "https.get"
        )
    {
        SinkKind::Network
    } else {
        return None;
    };
    Some(Sink { kind, args })
}

fn callee_is_or_ends_with(callee: &str, names: &[&str]) -> bool {
    names
        .iter()
        .any(|name| callee == *name || callee.ends_with(&format!(".{name}")))
}

pub(super) static JS_FAMILY_REVIEW: ReviewTables = ReviewTables {
    boundary: Some(&BoundaryKinds {
        decorator_kinds: &["decorator"],
        import_kinds: &["import_statement", "export_statement", "call_expression"],
    }),
    algorithmic: &AlgorithmicKinds {
        function_kinds: &[
            "function_declaration",
            "generator_function_declaration",
            "method_definition",
        ],
        loop_kinds: &[
            "for_statement",
            "for_in_statement",
            "for_of_statement",
            "while_statement",
            "do_statement",
        ],
        call_kinds: &["call_expression"],
        control_flow_kinds: &[
            "if_statement",
            "for_statement",
            "for_in_statement",
            "for_of_statement",
            "while_statement",
            "do_statement",
            "switch_statement",
            "try_statement",
        ],
        if_kinds: &["if_statement"],
    },
    removed: Some(&JS_FAMILY_REMOVED),
};

pub(super) static JS_FAMILY_REMOVED: RemovedTables = RemovedTables {
    extensions: &["js", "mjs", "cjs", "ts", "mts", "cts", "tsx", "jsx"],
    is_test_case: |node, content| {
        node.kind() == "call_expression"
            && node
                .child_by_field_name("function")
                .and_then(|callee| callee.utf8_text(content.as_bytes()).ok())
                .is_some_and(|callee| {
                    let callee = callee.trim();
                    callee == "test" || callee == "it" || callee == "describe"
                })
    },
    is_error_handling: |node, _| node.kind() == "try_statement",
    auth_call_kinds: &["call_expression"],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
    use crate::review::signals::content::ReviewSource;
    use crate::review::signals::taint::{SourceKind, TaintSignal, detect_taint};
    use std::path::PathBuf;

    #[test]
    fn nextjs_search_params_concatenated_into_sql_is_flagged() {
        let signals = run(r#"
export async function GET(request: NextRequest) {
  const id = request.nextUrl.searchParams.get("id");
  return db.query("SELECT * FROM users WHERE id = " + id);
}
"#);

        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].source, SourceKind::HttpRequest);
        assert_eq!(signals[0].sink, SinkKind::Sql);
    }

    #[test]
    fn nextjs_request_json_reaching_fs_write_is_flagged() {
        let signals = run(r#"
export async function POST(request: Request) {
  const body = await request.json();
  fs.writeFile(body.filename, "content", () => {});
}
"#);

        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].source, SourceKind::HttpRequest);
        assert_eq!(signals[0].sink, SinkKind::FsWrite);
    }

    #[test]
    fn nextjs_parameterized_query_is_not_flagged() {
        let signals = run(r#"
export async function GET(request: NextRequest) {
  const id = request.nextUrl.searchParams.get("id");
  return db.query("SELECT * FROM users WHERE id = $1", [id]);
}
"#);

        assert!(signals.is_empty(), "parameterized App Router query is safe");
    }

    #[test]
    fn response_json_is_not_treated_as_request_input() {
        let signals = run(r#"
async function transform(response: Response) {
  const body = await response.json();
  fs.writeFile(body.filename, "content", () => {});
}
"#);

        assert!(
            signals.is_empty(),
            "outbound response bodies are not request sources"
        );
    }

    #[test]
    fn fastify_raw_url_concatenated_into_sql_is_flagged() {
        let signals = run(r#"
async function audit(request: FastifyRequest) {
  const origin = request.raw.url;
  return db.query("SELECT * FROM audit_logs WHERE origin = '" + origin + "'");
}
"#);

        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].source, SourceKind::HttpRequest);
        assert_eq!(signals[0].sink, SinkKind::Sql);
    }

    #[test]
    fn fastify_proxy_metadata_reaching_fs_write_is_flagged() {
        for source in [
            "request.ip",
            "request.ips[0]",
            "request.hostname",
            "request.protocol",
        ] {
            let code = format!(
                r#"
async function persist(request: FastifyRequest) {{
  const value = {source};
  fs.writeFile(value, "content", () => {{}});
}}
"#
            );
            let signals = run(&code);

            assert_eq!(signals.len(), 1, "Fastify source {source} must propagate");
            assert_eq!(signals[0].source, SourceKind::HttpRequest);
            assert_eq!(signals[0].sink, SinkKind::FsWrite);
        }
    }

    #[test]
    fn fastify_parameterized_query_is_not_flagged() {
        let signals = run(r#"
async function audit(request: FastifyRequest) {
  const origin = request.raw.url;
  return db.query("SELECT * FROM audit_logs WHERE origin = $1", [origin]);
}
"#);

        assert!(signals.is_empty(), "parameterized Fastify query is safe");
    }

    #[test]
    fn fastify_reply_send_is_not_treated_as_request_input() {
        let signals = run(r#"
async function send(reply: FastifyReply) {
  const payload = reply.send({ filename: "report.txt" });
  fs.writeFile(payload.filename, "content", () => {});
}
"#);

        assert!(
            signals.is_empty(),
            "Fastify response serialization is not a request source"
        );
    }

    #[test]
    fn hono_query_concatenated_into_sql_is_flagged() {
        let signals = run(r#"
app.get("/users", async (c) => {
  const id = c.req.query("id");
  return db.query("SELECT * FROM users WHERE id = " + id);
});
"#);

        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].source, SourceKind::HttpRequest);
        assert_eq!(signals[0].sink, SinkKind::Sql);
    }

    #[test]
    fn hono_request_values_reaching_fs_write_are_flagged() {
        for source in [
            "c.req.param(\"name\")",
            "c.req.header(\"x-file\")",
            "await c.req.json()",
            "await c.req.parseBody()",
            "await c.req.text()",
            "c.req.url",
        ] {
            let code = format!(
                r#"
app.post("/write", async (c) => {{
  const value = {source};
  fs.writeFile(value, "content", () => {{}});
}});
"#
            );
            let signals = run(&code);

            assert_eq!(signals.len(), 1, "Hono source {source} must propagate");
            assert_eq!(signals[0].source, SourceKind::HttpRequest);
            assert_eq!(signals[0].sink, SinkKind::FsWrite);
        }
    }

    #[test]
    fn hono_parameterized_query_is_not_flagged() {
        let signals = run(r#"
app.get("/users", async (c) => {
  const id = c.req.query("id");
  return db.query("SELECT * FROM users WHERE id = $1", [id]);
});
"#);

        assert!(signals.is_empty(), "parameterized Hono query is safe");
    }

    #[test]
    fn hono_context_json_is_not_treated_as_request_input() {
        let signals = run(r#"
app.get("/report", async (c) => {
  const response = c.json({ filename: "report.txt" });
  fs.writeFile(response.filename, "content", () => {});
});
"#);

        assert!(
            signals.is_empty(),
            "Hono response serialization is not a request source"
        );
    }

    fn run(code: &str) -> Vec<TaintSignal> {
        let file = ChangedFile {
            path: PathBuf::from("src/routes/audit.ts"),
            status: ChangeStatus::Modified,
            ranges: vec![ChangedRange {
                start: 1,
                end: 100_000,
            }],
            hunks: Vec::new(),
        };
        let source = ReviewSource::new(code.to_string(), Some("TypeScript".to_string()));
        detect_taint(&file, Some(&source))
    }
}
