use super::{SinkKind, run};

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
fn python_unknown_call_preserves_taint_in_its_argument() {
    let signals = run(
        "src/handler.py",
        "Python",
        "def handler(request):\n    value = unknown(request.args.get('id'))\n    cursor.execute('SELECT * FROM users WHERE id=' + value)\n",
    );
    assert_eq!(
        signals.len(),
        1,
        "unknown calls must not erase argument taint"
    );
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

#[test]
fn python_taint_reaches_a_sink_through_one_local_helper_boundary() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
def execute(query):
    cursor.execute("SELECT * FROM users WHERE id = " + query)

def handler():
    query = request.args.get("id")
    execute(query)
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Sql);
    assert!(signals[0].detail.contains("parameter `query`"));
}

#[test]
fn python_subprocess_argument_vector_remains_safe_through_a_helper_boundary() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
def run_tool(value):
    subprocess.run(["grep", value])

def handler():
    run_tool(request.args.get("pattern"))
"#,
    );
    assert!(
        signals.is_empty(),
        "separate subprocess arguments are not shell text"
    );
}

#[test]
fn python_explicit_shell_command_remains_tainted_through_a_helper_boundary() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
def run_shell(value):
    subprocess.run(["sh", "-c", value])

def handler():
    run_shell(request.args.get("command"))
"#,
    );
    assert_eq!(signals.len(), 1, "nested shell execution remains a sink");
}

#[test]
fn python_combined_shell_flags_remain_tainted_through_a_helper_boundary() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
def run_shell(value):
    subprocess.run(["bash", "-lc", value])

def handler():
    run_shell(request.args.get("command"))
"#,
    );
    assert_eq!(signals.len(), 1, "bash -lc executes a shell command");
}

#[test]
fn python_unknown_helper_return_flow_preserves_argument_taint() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
def trim_query(value):
    return value.strip()

def handler():
    query = trim_query(request.args.get("id"))
    cursor.execute("SELECT * FROM users WHERE id = " + query)
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
fn python_local_identity_return_taint_survives_a_caller_alias() {
    let signals = run(
        "src/handler.py",
        "Python",
        r#"
def identity(value):
    return value

def handler():
    query = identity(request.args.get("id"))
    copy = query
    cursor.execute("SELECT * FROM users WHERE id=" + copy)
"#,
    );
    assert_eq!(signals.len(), 1, "helper return taint reaches aliases");
    assert_eq!(signals[0].sink, SinkKind::Sql);
}

#[test]
fn python_shell_command_flag_after_an_ordinary_option_remains_tainted() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
def run_shell(value):
    subprocess.run(["bash", "-l", "-c", value])

def handler():
    run_shell(request.args.get("command"))
"#,
    );
    assert_eq!(signals.len(), 1, "bash -l -c executes a shell command");
}

#[test]
fn python_powershell_encoded_command_remains_tainted() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
def run_shell(value):
    subprocess.run(["pwsh", "-NoProfile", "-EncodedCommand", value])

def handler():
    run_shell(request.args.get("command"))
"#,
    );
    assert_eq!(
        signals.len(),
        1,
        "encoded PowerShell command remains a sink"
    );
}

#[test]
fn python_shell_option_keeps_the_argument_vector_tainted() {
    let signals = run(
        "src/views.py",
        "Python",
        r#"
def run_tool(value):
    subprocess.run(["grep", value], shell=True)

def handler():
    run_tool(request.args.get("pattern"))
"#,
    );
    assert_eq!(
        signals.len(),
        1,
        "shell=True converts argv into shell input"
    );
}
