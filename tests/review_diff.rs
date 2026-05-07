use repopilot::review::diff::{ChangeStatus, parse_diff};

#[test]
fn parses_added_and_modified_hunk_ranges() {
    let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
index 1111111..2222222 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,0 +2,2 @@
+// TODO: review this
+pub fn added() {}
@@ -8 +10 @@
-old
+new
";

    let files = parse_diff(diff);

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].status, ChangeStatus::Modified);
    assert_eq!(files[0].path.to_string_lossy(), "src/lib.rs");
    assert_eq!(files[0].ranges[0].start, 2);
    assert_eq!(files[0].ranges[0].end, 3);
    assert_eq!(files[0].ranges[1].start, 10);
    assert_eq!(files[0].ranges[1].end, 10);
}

#[test]
fn parses_new_files() {
    let diff = "\
diff --git a/src/new.rs b/src/new.rs
new file mode 100644
index 0000000..1111111
--- /dev/null
+++ b/src/new.rs
@@ -0,0 +1,2 @@
+pub fn live() {}
+// FIXME: fix this
";

    let files = parse_diff(diff);

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].status, ChangeStatus::Added);
    assert_eq!(files[0].path.to_string_lossy(), "src/new.rs");
    assert_eq!(files[0].ranges[0].start, 1);
    assert_eq!(files[0].ranges[0].end, 2);
}

#[test]
fn parses_deleted_files_without_added_ranges() {
    let diff = "\
diff --git a/src/old.rs b/src/old.rs
deleted file mode 100644
index 1111111..0000000
--- a/src/old.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-pub fn old() {}
-// TODO: remove
";

    let files = parse_diff(diff);

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].status, ChangeStatus::Deleted);
    assert_eq!(files[0].path.to_string_lossy(), "src/old.rs");
    assert!(files[0].ranges.is_empty());
}

#[test]
fn parses_renamed_files_with_new_path() {
    let diff = "\
diff --git a/src/old.rs b/src/new.rs
similarity index 80%
rename from src/old.rs
rename to src/new.rs
index 1111111..2222222 100644
--- a/src/old.rs
+++ b/src/new.rs
@@ -4,0 +5 @@
+// HACK: temporary
";

    let files = parse_diff(diff);

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].status, ChangeStatus::Renamed);
    assert_eq!(files[0].path.to_string_lossy(), "src/new.rs");
    assert_eq!(files[0].ranges[0].start, 5);
    assert_eq!(files[0].ranges[0].end, 5);
}

#[test]
fn ignores_zero_line_added_hunks() {
    let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
index 1111111..2222222 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -4,2 +4,0 @@
-old
-lines
";

    let files = parse_diff(diff);

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].status, ChangeStatus::Modified);
    assert!(files[0].ranges.is_empty());
}
