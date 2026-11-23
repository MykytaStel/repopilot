use super::super::detect_taint;
use super::run;
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
use crate::review::signals::content::ReviewSource;
use std::path::PathBuf;

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
