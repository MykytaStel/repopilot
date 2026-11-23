use super::{SinkKind, run};

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
fn go_unknown_call_preserves_taint_in_its_argument() {
    let signals = run(
        "src/handler.go",
        "Go",
        "package app\nfunc handler(r *http.Request) { value := unknown(r.URL.Query().Get(\"id\")); db.Query(\"SELECT * FROM users WHERE id=\" + value) }\n",
    );
    assert_eq!(
        signals.len(),
        1,
        "unknown calls must not erase argument taint"
    );
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

#[test]
fn go_taint_reaches_a_sink_through_one_local_helper_boundary() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

func execute(db *DB, query string) {
	db.Query("SELECT * FROM users WHERE id = " + query)
}

func handler(r *Request, db *DB) {
	id := r.URL.Query().Get("id")
	execute(db, id)
}
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Sql);
    assert!(signals[0].detail.contains("parameter `query`"));
}

#[test]
fn go_grouped_helper_parameters_keep_their_argument_positions() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

func execute(query, table string) {
	db.Query("SELECT * FROM " + table + " WHERE id = " + query)
}

func handler(r *Request) {
	execute(r.URL.Query().Get("id"), r.URL.Query().Get("table"))
}
"#,
    );
    assert_eq!(
        signals.len(),
        1,
        "grouped Go parameters retain taint summaries"
    );
    assert_eq!(signals[0].sink, SinkKind::Sql);
}

#[test]
fn go_unknown_helper_return_flow_preserves_argument_taint() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

func trimQuery(value string) string { return strings.TrimSpace(value) }
func handler(r *Request) {
	query := trimQuery(r.URL.Query().Get("id"))
	db.Query("SELECT * FROM users WHERE id = " + query)
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
fn go_local_identity_return_taint_survives_a_caller_alias() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

func identity(value string) string { return value }
func handler(r *Request) {
	query := identity(r.URL.Query().Get("id"))
	copy := query
	db.Query("SELECT * FROM users WHERE id=" + copy)
}
"#,
    );
    assert_eq!(signals.len(), 1, "helper return taint reaches aliases");
    assert_eq!(signals[0].sink, SinkKind::Sql);
}

#[test]
fn go_exec_command_argument_vector_remains_safe_through_a_helper_boundary() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

import "os/exec"

func runTool(value string) { exec.Command("grep", value) }
func handler(r *Request) { runTool(r.URL.Query().Get("pattern")) }
"#,
    );
    assert!(
        signals.is_empty(),
        "separate exec.Command arguments are not shell text"
    );
}

#[test]
fn go_explicit_shell_command_remains_tainted_through_a_helper_boundary() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

import "os/exec"

func runShell(value string) { exec.Command("sh", "-c", value) }
func handler(r *Request) { runShell(r.URL.Query().Get("command")) }
"#,
    );
    assert_eq!(signals.len(), 1, "nested shell execution remains a sink");
}

#[test]
fn go_combined_shell_flags_remain_tainted_through_a_helper_boundary() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

import "os/exec"

func runShell(value string) { exec.Command("bash", "-lc", value) }
func handler(r *Request) { runShell(r.URL.Query().Get("command")) }
"#,
    );
    assert_eq!(signals.len(), 1, "bash -lc executes a shell command");
}

#[test]
fn go_shell_command_flag_after_an_ordinary_option_remains_tainted() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

import "os/exec"

func runShell(value string) { exec.Command("bash", "-l", "-c", value) }
func handler(r *Request) { runShell(r.URL.Query().Get("command")) }
"#,
    );
    assert_eq!(signals.len(), 1, "bash -l -c executes a shell command");
}

#[test]
fn go_powershell_encoded_command_remains_tainted() {
    let signals = run(
        "src/handler.go",
        "Go",
        r#"
package main

import "os/exec"

func runShell(value string) { exec.Command("pwsh", "-NoProfile", "-EncodedCommand", value) }
func handler(r *Request) { runShell(r.URL.Query().Get("command")) }
"#,
    );
    assert_eq!(
        signals.len(),
        1,
        "encoded PowerShell command remains a sink"
    );
}
