use super::{SinkKind, run, run_with_changed_lines};

#[test]
fn js_taint_reaches_a_sink_through_one_local_helper_boundary() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function run_query(query: string) {
  db.query("SELECT * FROM users WHERE id = " + query);
}
function handler() {
  const id = req.query.id;
  run_query(id);
}
"#,
    );
    assert_eq!(signals.len(), 1, "{signals:?}");
    assert_eq!(signals[0].sink, SinkKind::Sql);
    assert!(signals[0].detail.contains("parameter `query`"));
}

#[test]
fn js_sanitized_helper_argument_does_not_reach_a_sink() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function run_query(query: string) {
  db.query("SELECT * FROM users WHERE id = " + query);
}
function handler() {
  run_query(parseInt(req.query.id, 10));
}
"#,
    );
    assert!(
        signals.is_empty(),
        "recognized coercions remain sanitizers across a call"
    );
}

#[test]
fn js_recursive_duplicate_and_dynamic_helpers_are_not_summarized() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function recursive(value: string) {
  if (value) recursive(value);
  db.query("SELECT " + value);
}
function duplicate(value: string) { db.query("SELECT " + value); }
function duplicate(value: string) { db.query("SELECT " + value); }
function handler(service: { run: (value: string) => void }) {
  recursive(req.query.id);
  duplicate(req.query.id);
  service.run(req.query.id);
}
"#,
    );
    assert!(
        signals.is_empty(),
        "ambiguous and dynamic helper targets abstain"
    );
}

#[test]
fn js_local_identity_helper_return_reaches_the_caller_sink() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function identity(value: string) { return value; }
function handler() {
  const query = identity(req.query.id);
  db.query("SELECT * FROM users WHERE id = " + query);
}
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Sql);
    assert!(signals[0].detail.contains("returns tainted parameter"));
}

#[test]
fn js_local_identity_return_taint_survives_a_caller_alias() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function identity(value: string) { return value; }
function handler() {
  const query = identity(req.query.id);
  const copy = query;
  db.query("SELECT * FROM users WHERE id = " + copy);
}
"#,
    );
    assert_eq!(signals.len(), 1, "helper return taint reaches aliases");
    assert_eq!(signals[0].sink, SinkKind::Sql);
}

#[test]
fn js_changed_helper_call_reports_unchanged_helper_sink() {
    let signals = run_with_changed_lines(
        "src/handler.ts",
        "TypeScript",
        r#"
function run_query(query: string) {
  db.query("SELECT * FROM users WHERE id = " + query);
}
function handler() {
  const id = req.query.id;
  run_query(id);
}
"#,
        7,
        7,
    );
    assert_eq!(
        signals.len(),
        1,
        "changed call reaches unchanged helper sink"
    );
    assert_eq!(signals[0].sink, SinkKind::Sql);
    assert_eq!(signals[0].line, 3, "location remains the sink line");
    assert!(signals[0].detail.contains("at line 7"));
}

#[test]
fn js_changed_helper_return_reaches_unchanged_caller_alias_sink() {
    let signals = run_with_changed_lines(
        "src/handler.ts",
        "TypeScript",
        r#"
function identity(value: string) { return value; }
function handler() {
  const query = identity(req.query.id);
  const copy = query;
  db.query("SELECT * FROM users WHERE id = " + copy);
}
"#,
        4,
        4,
    );
    assert_eq!(signals.len(), 1, "helper return reaches caller alias");
    assert_eq!(signals[0].sink, SinkKind::Sql);
    assert_eq!(signals[0].line, 6, "location remains the sink line");
    assert!(signals[0].detail.contains("returns tainted parameter"));
}

#[test]
fn js_helper_return_does_not_retaint_a_cleaned_parameter() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function clean(value: string) { value = "fixed"; return value; }
function handler() {
  const query = clean(req.query.id);
  db.query("SELECT * FROM users WHERE id = " + query);
}
"#,
    );
    assert!(
        signals.is_empty(),
        "clean helper must not return inherited taint"
    );
}

#[test]
fn js_unknown_helper_return_flow_preserves_argument_taint() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function trimQuery(value: string) { return value.trim(); }
function handler() {
  const query = trimQuery(req.query.id);
  db.query("SELECT * FROM users WHERE id = " + query);
}
"#,
    );
    assert_eq!(
        signals.len(),
        1,
        "unknown transformations must preserve taint"
    );
    assert_eq!(signals[0].sink, SinkKind::Sql);
}

#[test]
fn js_helper_return_does_not_taint_sinks_before_the_call() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function identity(value: string) { return value; }
function handler() {
  let query = "fixed";
  db.query("SELECT * FROM users WHERE id = " + query);
  query = identity(req.query.id);
}
"#,
    );
    assert!(
        signals.is_empty(),
        "a later helper call cannot taint an earlier sink"
    );
}

#[test]
fn js_parameterized_query_remains_safe_through_a_helper_boundary() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function run_query(query: string) {
  db.query("SELECT * FROM users WHERE id = $1", [query]);
}
function handler() {
  run_query(req.query.id);
}
"#,
    );
    assert!(signals.is_empty(), "bound SQL arguments remain safe");
}

#[test]
fn js_unknown_call_preserves_taint_in_its_argument() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function handler() {
  const result = unknown(req.query.id);
  db.query("SELECT " + result);
}
"#,
    );
    assert_eq!(
        signals.len(),
        1,
        "unknown calls must not erase argument taint"
    );
}

#[test]
fn js_clean_helper_overwrite_does_not_transfer_taint() {
    let signals = run(
        "src/handler.ts",
        "TypeScript",
        r#"
function clean(value: string) { value = "fixed"; db.query("SELECT " + value); }
function handler() { clean(req.query.id); }
"#,
    );
    assert!(signals.is_empty());
}
