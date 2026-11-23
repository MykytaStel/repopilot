use super::{SinkKind, run};

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
