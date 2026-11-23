use super::{SinkKind, SourceKind, run};

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
