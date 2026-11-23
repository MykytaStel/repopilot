use super::{SinkKind, SourceKind, run};

// ── JavaScript / TypeScript ───────────────────────────────────────────────────

#[test]
fn js_request_concatenated_into_sql_is_flagged() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
const id = req.query.id;
db.query("SELECT * FROM users WHERE id = " + id);
"#,
    );
    assert_eq!(signals.len(), 1, "{signals:?}");
    assert_eq!(signals[0].sink, SinkKind::Sql);
    assert_eq!(signals[0].source, SourceKind::HttpRequest);
}

#[test]
fn js_request_interpolated_into_sql_is_flagged() {
    let signals = run(
        "src/handler.js",
        "JavaScript",
        "db.query(`SELECT * FROM t WHERE id = ${req.params.id}`);\n",
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Sql);
}

#[test]
fn js_parameterized_query_is_not_flagged() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
const id = req.query.id;
db.query("SELECT * FROM users WHERE id = $1", [id]);
"#,
    );
    assert!(signals.is_empty(), "parameterized query must be safe");
}

#[test]
fn js_static_query_is_not_flagged() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        "db.query(\"SELECT * FROM users WHERE active = true\");\n",
    );
    assert!(signals.is_empty());
}

#[test]
fn js_request_into_exec_is_flagged() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
const name = req.body.name;
exec("ls " + name);
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Exec);
}

#[test]
fn js_unrelated_local_named_like_a_source_is_not_flagged() {
    // `valid` must not be read as the tainted name `id` — whole-token matching.
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
const valid = computeFlag();
db.query("SELECT * FROM t WHERE ok = " + valid);
"#,
    );
    assert!(signals.is_empty());
}

#[test]
fn js_source_name_inside_a_string_is_not_flagged() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
const example = "req.query.id";
exec(example);
"#,
    );
    assert!(signals.is_empty());
}

#[test]
fn js_tainted_local_name_inside_a_string_is_not_flagged() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
const id = req.query.id;
exec("id");
"#,
    );
    assert!(signals.is_empty());
}

#[test]
fn js_source_inside_a_callback_does_not_taint_the_callback_value() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
const callback = () => req.query.id;
exec(callback);
"#,
    );
    assert!(signals.is_empty());
}

#[test]
fn js_process_argv_into_fs_write_is_flagged() {
    let signals = run(
        "src/cli.ts",
        "TypeScript",
        r#"
const output = process.argv[2];
fs.writeFile(output, "content", () => {});
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].source, SourceKind::ProcessArgs);
    assert_eq!(signals[0].sink, SinkKind::FsWrite);
}

#[test]
fn js_taint_does_not_leak_between_functions() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function readsInput() {
  const id = req.query.id;
  return id;
}

function runsQuery(id: string) {
  db.query("SELECT * FROM t WHERE id = " + id);
}
"#,
    );
    assert!(signals.is_empty());
}

#[test]
fn js_clean_reassignment_clears_taint() {
    // `id` is tainted, then overwritten with a constant before the sink.
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
let id = req.query.id;
id = "static";
db.query("SELECT * FROM t WHERE id = " + id);
"#,
    );
    assert!(signals.is_empty(), "a clean reassignment must clear taint");
}

#[test]
fn js_reassignment_to_a_source_starts_tainting() {
    // The reverse: clean first, then reassigned from a request field.
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
let id = "static";
id = req.query.id;
db.query("SELECT * FROM t WHERE id = " + id);
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Sql);
}
