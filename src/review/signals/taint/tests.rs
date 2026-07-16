use super::{SinkKind, SourceKind, detect_taint};
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
use crate::review::signals::content::ReviewSource;
use std::path::PathBuf;

/// Run taint detection over `code`, treating every line as changed so the sink is
/// always in-diff. `path` drives language detection; `label` drives parsing.
fn run(path: &str, label: &str, code: &str) -> Vec<super::TaintSignal> {
    let file = ChangedFile {
        path: PathBuf::from(path),
        status: ChangeStatus::Modified,
        ranges: vec![ChangedRange {
            start: 1,
            end: 100_000,
        }],
        hunks: Vec::new(),
    };
    let source = ReviewSource::new(code.to_string(), Some(label.to_string()));
    detect_taint(&file, Some(&source))
}

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
    assert_eq!(signals.len(), 1);
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

// ── Python ────────────────────────────────────────────────────────────────────

#[test]
fn python_request_formatted_into_sql_is_flagged() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
user = request.args.get("id")
cursor.execute("SELECT * FROM t WHERE id = " + user)
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Sql);
}

#[test]
fn python_fstring_into_sql_is_flagged() {
    let signals = run(
        "src/views.py",
        "Python",
        "user = request.args.get(\"id\")\ncursor.execute(f\"SELECT * FROM t WHERE id = {user}\")\n",
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Sql);
}

#[test]
fn python_parameterized_query_is_not_flagged() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
user = request.args.get("id")
cursor.execute("SELECT * FROM t WHERE id = %s", (user,))
"#,
    );
    assert!(signals.is_empty());
}

#[test]
fn python_request_into_subprocess_is_flagged() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
name = request.args.get("name")
subprocess.run("ls " + name, shell=True)
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Exec);
}

#[test]
fn python_request_into_network_is_flagged() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
url = request.args.get("url")
requests.get(url)
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Network);
}

#[test]
fn python_read_only_open_is_not_flagged_as_a_write() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
path = request.args.get("path")
open(path, "r")
"#,
    );
    assert!(signals.is_empty());
}

#[test]
fn python_write_open_is_flagged() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
path = request.args.get("path")
open(path, mode="w")
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::FsWrite);
}

#[test]
fn python_compound_assignment_keeps_taint() {
    // `q += " more"` combines with the tainted value, so taint must NOT clear.
    let signals = run(
        "src/views.py",
        "Python",
        r#"
q = request.args.get("id")
q += " ORDER BY 1"
cursor.execute("SELECT * FROM t WHERE id = " + q)
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Sql);
}

// ── Go ────────────────────────────────────────────────────────────────────────

#[test]
fn go_request_concatenated_into_sql_is_flagged() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

func handler(r *Request, db *DB) {
	id := r.URL.Query().Get("id")
	rows, _ := db.Query("SELECT * FROM t WHERE id = " + id)
	_ = rows
}
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Sql);
}

#[test]
fn go_parameterized_query_is_not_flagged() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

func handler(r *Request, db *DB) {
	id := r.URL.Query().Get("id")
	rows, _ := db.Query("SELECT * FROM t WHERE id = $1", id)
	_ = rows
}
"#,
    );
    assert!(signals.is_empty());
}

#[test]
fn go_server_api_is_not_flagged_as_outbound_network() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

func handler(r *Request) {
	http.Handle(r.URL.Query().Get("path"), nil)
}
"#,
    );
    assert!(signals.is_empty());
}

// ── Coercion / sanitizers ──────────────────────────────────────────────────────

#[test]
fn js_numeric_coercion_clears_taint() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
const id = Number(req.query.id);
db.query("SELECT * FROM t WHERE id = " + id);
"#,
    );
    assert!(
        signals.is_empty(),
        "Number(...) coercion neutralizes injection"
    );
}

#[test]
fn js_parseint_coercion_into_exec_is_not_flagged() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
const n = parseInt(req.query.n, 10);
exec("kill " + n);
"#,
    );
    assert!(signals.is_empty());
}

#[test]
fn js_partial_coercion_still_flags_the_raw_part() {
    // Number(a) is neutralized, but b is still raw — pruning is subtree-local.
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
const a = req.query.a;
const b = req.query.b;
db.query("SELECT * FROM t WHERE a = " + Number(a) + " AND b = " + b);
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Sql);
}

#[test]
fn python_int_coercion_clears_taint() {
    let signals = run(
        "src/views.py",
        "Python",
        "uid = int(request.args.get(\"id\"))\ncursor.execute(f\"SELECT * FROM t WHERE id = {uid}\")\n",
    );
    assert!(signals.is_empty());
}

#[test]
fn go_strconv_atoi_clears_taint() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

func handler(r *Request, db *DB) {
	id, _ := strconv.Atoi(r.URL.Query().Get("id"))
	rows, _ := db.Query(fmt.Sprintf("SELECT * FROM t WHERE id = %d", id))
	_ = rows
}
"#,
    );
    assert!(signals.is_empty());
}

// ── Java ──────────────────────────────────────────────────────────────────────

#[test]
fn java_request_into_exec_is_flagged() {
    let signals = run(
        "src/main/java/App.java",
        "Java",
        r#"
class App {
    void handle(HttpServletRequest request) throws Exception {
        String cmd = request.getParameter("cmd");
        Runtime.getRuntime().exec("convert " + cmd);
    }
}
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Exec);
    assert_eq!(signals[0].source, SourceKind::HttpRequest);
}

#[test]
fn java_request_concatenated_into_sql_is_flagged() {
    let signals = run(
        "src/main/java/App.java",
        "Java",
        r#"
class App {
    void find(HttpServletRequest request, Statement stmt) throws Exception {
        String id = request.getParameter("id");
        stmt.executeQuery("SELECT * FROM owners WHERE id = " + id);
    }
}
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Sql);
}

#[test]
fn java_parameterized_query_is_not_flagged() {
    // A static query string with the value bound as a parameter is the safe,
    // parameterized pattern and must not flag.
    let signals = run(
        "src/main/java/App.java",
        "Java",
        r#"
class App {
    void find(HttpServletRequest request, PreparedStatement ps) throws Exception {
        String id = request.getParameter("id");
        ps.setString(1, id);
        ps.executeQuery();
    }
}
"#,
    );
    assert!(signals.is_empty());
}

#[test]
fn java_clean_request_read_without_sink_is_not_flagged() {
    let signals = run(
        "src/main/java/App.java",
        "Java",
        r#"
class App {
    String greet(HttpServletRequest request) {
        String name = request.getParameter("name");
        return "Hello " + name;
    }
}
"#,
    );
    assert!(signals.is_empty());
}

// ── Scope guards ──────────────────────────────────────────────────────────────

#[test]
fn test_files_are_skipped() {
    let signals = run(
        "src/handler.test.ts",
        "TypeScript",
        r#"
const id = req.query.id;
db.query("SELECT * FROM users WHERE id = " + id);
"#,
    );
    assert!(signals.is_empty());
}

#[test]
fn sink_outside_changed_lines_is_not_flagged() {
    let file = ChangedFile {
        path: PathBuf::from("src/handler.ts"),
        status: ChangeStatus::Modified,
        // Only line 2 is "changed"; the sink sits on line 3.
        ranges: vec![ChangedRange { start: 2, end: 2 }],
        hunks: Vec::new(),
    };
    let code = "\nconst id = req.query.id;\ndb.query(\"SELECT * FROM t WHERE id = \" + id);\n";
    let source = ReviewSource::new(code.to_string(), Some("TypeScript".to_string()));
    assert!(detect_taint(&file, Some(&source)).is_empty());
}
