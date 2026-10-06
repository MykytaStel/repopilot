use super::detect_taint;
use super::tests::run;
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
use crate::review::signals::content::ReviewSource;
use std::path::PathBuf;

fn code(statements: &str) -> String {
    format!(
        "class Controller {{ void Find(SqlCommand cmd) {{\nvar id = Request.Query[\"id\"];\n{statements}\n}} }}"
    )
}

#[test]
fn csharp_command_property_controls() {
    for statements in [
        "cmd.CommandText = \"SELECT * WHERE id = @id\"; cmd.Parameters.AddWithValue(\"@id\", id); cmd.ExecuteReader();",
        "cmd.CommandText = id; cmd.CommandText = \"SELECT 1\"; cmd.ExecuteReader();",
        "cmd.CommandText = id; other.ExecuteReader();",
        "cmd.CommandText = id; cmd = otherCommand; cmd.ExecuteReader();",
        "cmd.CommandText = id; cmd = new SqlCommand(); cmd.ExecuteReader();",
        "cmd.CommandText = id;",
        "if (ok) { cmd.CommandText = id; } cmd.ExecuteReader();",
        "cmd.CommandText = id; if (ok) { cmd.CommandText = \"SELECT 1\"; } cmd.ExecuteReader();",
        "{ cmd.CommandText = id; } cmd.ExecuteReader();",
        "cmd.CommandText = id; void Inner() { cmd.ExecuteReader(); }",
    ] {
        assert!(
            run("src/Controller.cs", "C#", &code(statements)).is_empty(),
            "{statements}"
        );
    }
}

#[test]
fn csharp_command_property_execution_family_and_compound() {
    for method in [
        "ExecuteReader",
        "ExecuteNonQuery",
        "ExecuteScalar",
        "ExecuteReaderAsync",
        "ExecuteNonQueryAsync",
        "ExecuteScalarAsync",
    ] {
        for assignment in [
            "cmd.CommandText = id;",
            "cmd.CommandText = id; cmd.CommandText += \" ORDER BY id\";",
            "cmd.CommandText = \"SELECT \"; cmd.CommandText += id;",
            "cmd.CommandText = id; cmd.Parameters.AddWithValue(\"@id\", id);",
        ] {
            let signals = run(
                "src/Controller.cs",
                "C#",
                &code(&format!("{assignment}\ncmd.{method}();")),
            );
            assert_eq!(signals.len(), 1, "{assignment} {method}");
            assert!(signals[0].detail.contains("at line 2"));
            assert!(signals[0].detail.contains("CommandText at line 3"));
            assert!(signals[0].detail.contains(method));
        }
    }
}

#[test]
fn csharp_changed_property_assignment_with_unchanged_execution() {
    let source = ReviewSource::new(
        code("cmd.CommandText = id;\ncmd.ExecuteReader();"),
        Some("C#".into()),
    );
    let mut file = ChangedFile {
        path: PathBuf::from("src/Controller.cs"),
        status: ChangeStatus::Modified,
        ranges: vec![ChangedRange { start: 3, end: 3 }],
        hunks: Vec::new(),
    };
    let signals = detect_taint(&file, Some(&source));
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].line, 4);
    file.ranges = vec![ChangedRange { start: 1, end: 1 }];
    assert!(detect_taint(&file, Some(&source)).is_empty());
}

#[test]
fn csharp_command_property_abstains_on_expression_order_ambiguity() {
    for statements in [
        "cmd.CommandText = id + cmd.ExecuteScalar(); cmd.ExecuteReader();",
        "cmd.CommandText = id; cmd.CommandText = cmd.ExecuteScalar().ToString(); cmd.ExecuteReader();",
    ] {
        assert!(
            run("src/Controller.cs", "C#", &code(statements)).is_empty(),
            "{statements}"
        );
    }
}

#[test]
fn csharp_command_property_abstains_on_conditional_mutation() {
    for statements in [
        "var ignored = ok ? (cmd.CommandText = id) : \"\"; cmd.ExecuteReader();",
        "cmd.CommandText = id; var ignored = ok ? (cmd.CommandText = \"SELECT 1\") : \"\"; cmd.ExecuteReader();",
        "var ignored = ok && ((cmd.CommandText = id) != null); cmd.ExecuteReader();",
        "var ignored = ok || ((cmd.CommandText = id) != null); cmd.ExecuteReader();",
        "var ignored = fallback ?? (cmd.CommandText = id); cmd.ExecuteReader();",
        "cmd.CommandText ??= id; cmd.ExecuteReader();",
    ] {
        assert!(
            run("src/Controller.cs", "C#", &code(statements)).is_empty(),
            "{statements}"
        );
    }
}

#[test]
fn csharp_expression_guard_preserves_direct_sql_argument_flow() {
    let signals = run(
        "src/Controller.cs",
        "C#",
        &code("var result = db.ExecuteScalar(\"SELECT * WHERE id = \" + id);"),
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].line, 3);
}
