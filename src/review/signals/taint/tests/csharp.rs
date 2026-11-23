use super::{SinkKind, SourceKind, run};

// ── C# ────────────────────────────────────────────────────────────────────────

#[test]
fn csharp_request_into_process_start_is_flagged() {
    let signals = run(
        "src/App/OrdersController.cs",
        "C#",
        r#"
class OrdersController {
    public void Export(HttpRequest request) {
        var format = Request.Query["format"];
        Process.Start("export.exe " + format);
    }
}
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sink, SinkKind::Exec);
    assert_eq!(signals[0].source, SourceKind::HttpRequest);
}

#[test]
fn csharp_request_interpolated_into_sql_is_flagged() {
    let signals = run(
        "src/App/OrdersController.cs",
        "C#",
        r#"
class OrdersController {
    public void Find(SqlCommand cmd) {
        var id = Request.Query["id"];
        cmd.CommandText = $"SELECT * FROM orders WHERE id = {id}";
        cmd.ExecuteReader();
    }
}
"#,
    );
    let direct = run(
        "src/App/OrdersController.cs",
        "C#",
        r#"
class OrdersController {
    public void Find(Db db) {
        var id = Request.Query["id"];
        db.ExecuteScalar("SELECT * FROM orders WHERE id = " + id);
    }
}
"#,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(direct.len(), 1);
    assert_eq!(direct[0].sink, SinkKind::Sql);
}

#[test]
fn csharp_clean_request_read_without_sink_is_not_flagged() {
    let signals = run(
        "src/App/OrdersController.cs",
        "C#",
        r#"
class OrdersController {
    public string Greet() {
        var name = Request.Query["name"];
        return "Hello " + name;
    }
}
"#,
    );
    assert!(signals.is_empty());
}
