use super::*;

#[test]
fn repopilot_internal_paths_are_excluded_from_review_scope() {
    assert!(is_repopilot_internal_path(Path::new(
        ".repopilot/cache/repo_context.json"
    )));
    assert!(is_repopilot_internal_path(Path::new(
        r".repopilot\cache\repo_context.json"
    )));
    assert!(!is_repopilot_internal_path(Path::new("src/lib.rs")));
}

#[test]
fn parses_modified_file_added_and_removed_lines() {
    let diff = "\
diff --git a/src/app.ts b/src/app.ts
index 1111111..2222222 100644
--- a/src/app.ts
+++ b/src/app.ts
@@ -10,2 +10,3 @@ fn main()
-old one
-old two
+new one
+new two
+new three
";

    let files = parse_diff(diff);
    assert_eq!(files.len(), 1);
    let file = &files[0];
    assert_eq!(file.path, PathBuf::from("src/app.ts"));
    assert_eq!(file.status, ChangeStatus::Modified);
    // `ranges` stays the public added-line view, derived from the hunk.
    assert_eq!(file.ranges, vec![ChangedRange { start: 10, end: 12 }]);
    assert_eq!(file.hunks.len(), 1);
    let hunk = &file.hunks[0];
    assert_eq!(hunk.new_range, Some(ChangedRange { start: 10, end: 12 }));
    assert_eq!(hunk.added_lines, vec!["new one", "new two", "new three"]);
    assert_eq!(hunk.removed_lines, vec!["old one", "old two"]);
}

#[test]
fn parses_added_file_content() {
    let diff = "\
diff --git a/src/new.ts b/src/new.ts
new file mode 100644
index 0000000..3333333
--- /dev/null
+++ b/src/new.ts
@@ -0,0 +1,2 @@
+hello
+world
";

    let files = parse_diff(diff);
    let file = &files[0];
    assert_eq!(file.path, PathBuf::from("src/new.ts"));
    assert_eq!(file.status, ChangeStatus::Added);
    assert_eq!(file.ranges, vec![ChangedRange { start: 1, end: 2 }]);
    assert_eq!(file.hunks[0].added_lines, vec!["hello", "world"]);
    assert!(file.hunks[0].removed_lines.is_empty());
}

#[test]
fn parses_deleted_file_removed_lines_with_no_range() {
    let diff = "\
diff --git a/src/old.ts b/src/old.ts
deleted file mode 100644
index 4444444..0000000
--- a/src/old.ts
+++ /dev/null
@@ -1,3 +0,0 @@
-line a
-line b
-line c
";

    let files = parse_diff(diff);
    let file = &files[0];
    assert_eq!(file.path, PathBuf::from("src/old.ts"));
    assert_eq!(file.status, ChangeStatus::Deleted);
    // A pure-deletion hunk contributes no added-line range, just like before.
    assert!(file.ranges.is_empty());
    assert_eq!(file.hunks[0].new_range, None);
    assert_eq!(
        file.hunks[0].removed_lines,
        vec!["line a", "line b", "line c"]
    );
}

#[test]
fn captures_multiple_hunks_independently() {
    let diff = "\
diff --git a/src/multi.ts b/src/multi.ts
index 5555555..6666666 100644
--- a/src/multi.ts
+++ b/src/multi.ts
@@ -1 +1 @@
-first old
+first new
@@ -20,0 +21,2 @@
+added a
+added b
";

    let files = parse_diff(diff);
    let file = &files[0];
    assert_eq!(file.hunks.len(), 2);
    assert_eq!(
        file.ranges,
        vec![
            ChangedRange { start: 1, end: 1 },
            ChangedRange { start: 21, end: 22 },
        ]
    );
    assert_eq!(file.hunks[0].added_lines, vec!["first new"]);
    assert_eq!(file.hunks[0].removed_lines, vec!["first old"]);
    assert_eq!(file.hunks[1].added_lines, vec!["added a", "added b"]);
    assert!(file.hunks[1].removed_lines.is_empty());
}
