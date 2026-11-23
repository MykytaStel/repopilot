//! Taint tables for the JS dialect family. JS/TS (and their React variants)
//! share a grammar shape, so one table covers both frontends. Source idioms
//! target Express/Koa, Next.js App Router, Fastify, and Hono; sinks mirror the
//! behavioral "added X" detectors.

mod sinks;
mod tables;
pub(super) use tables::{JS_FAMILY_REVIEW, JS_FAMILY_TAINT};

#[cfg(test)]
mod tests {
    use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
    use crate::review::signals::content::ReviewSource;
    use crate::review::signals::taint::sinks::SinkKind;
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
