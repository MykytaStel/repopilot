//! Test-integrity signals: a change that weakens the checks judging it.
//!
//! Skip and focus markers (`it.only`, `@pytest.mark.skip`, `t.Skip`,
//! `#[ignore]`), removed test cases, removed assertions, new lint, type, or
//! coverage suppressions, and relaxed CI or tool gates. Everything is
//! counted over the whole pre- and post-change file, not only the changed
//! lines, so re-indenting or moving code within a file is not a change. A
//! marker or test that disappears from one changed file and appears in another
//! is a move and is not reported either. A skip in a brand-new file weakens
//! nothing that ran before and is not reported; a focus marker is, because it
//! stops the rest of the suite.

mod accounting;
#[cfg(test)]
mod accounting_tests;
mod gates;
mod markers;
mod suppressions;
#[cfg(test)]
mod suppressions_tests;
pub(crate) mod syntax;
#[cfg(test)]
mod tests;

use crate::languages::integrity_for_extension;
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::signals::content::ReviewSource;
use crate::review::signals::tables::{IntegrityTables, TestMarker};
pub use gates::detect_gate_relaxation;
use serde::Serialize;
use tree_sitter::Node;

/// The category of test-integrity change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrityKind {
    TestFocused,
    TestSkipped,
    TestRemoved,
    AssertionsRemoved,
    SuppressionAdded,
    GateRelaxed,
}

/// A test-integrity signal detected in a changed file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegritySignal {
    pub kind: IntegrityKind,
    pub path: String,
    pub line: usize,
    pub detail: String,
}

/// The integrity evidence of one changed file, before and after the change.
#[derive(Debug)]
pub struct FileEvidence {
    path: String,
    existed_before: bool,
    /// Whole-file test removal (deleted or emptied test file) is reported by
    /// the behavioral `test-deleted-or-emptied` signal, not here.
    removal_reported_elsewhere: bool,
    first_changed_line: usize,
    pre: Scan,
    post: Scan,
    /// Post-change markers on changed lines, by index into `post.markers`.
    post_marker_in_diff: Vec<bool>,
}

/// What one side of a changed file contains. Markers and test cases are
/// collected only in test scope; suppressions everywhere.
#[derive(Debug, Default)]
struct Scan {
    markers: Vec<TestMarker>,
    tests: Vec<accounting::TestFacts>,
    suppressions: Vec<(String, usize)>,
}

/// Collects a changed file's evidence, or `None` when no recognizer applies or
/// a side that should exist cannot be parsed (never guess from half a picture).
pub fn collect_file_evidence(
    file: &ChangedFile,
    pre: Option<&ReviewSource>,
    post: Option<&ReviewSource>,
) -> Option<FileEvidence> {
    let ext = file.path.extension().and_then(|ext| ext.to_str())?;
    let tables = integrity_for_extension(ext)?;
    if is_test_input_data(&file.path) {
        return None;
    }
    let is_test_file = crate::audits::context::classify::helpers::is_test_file(&file.path);
    let test_scope = tables.applies_outside_test_files || is_test_file;
    let existed_before = !matches!(file.status, ChangeStatus::Added | ChangeStatus::Untracked);
    let pre = if existed_before {
        scan(pre?, tables, test_scope)?
    } else {
        Scan::default()
    };
    let post = match post {
        Some(source) => scan(source, tables, test_scope)?,
        None if file.status == ChangeStatus::Deleted => Scan::default(),
        None => return None,
    };
    Some(FileEvidence {
        path: file.path_string(),
        existed_before,
        removal_reported_elsewhere: is_test_file && post.tests.is_empty(),
        first_changed_line: file.ranges.first().map_or(1, |range| range.start.max(1)),
        post_marker_in_diff: post
            .markers
            .iter()
            .map(|marker| file.contains_line(marker.line))
            .collect(),
        pre,
        post,
    })
}

/// Reports every integrity signal the change produces, in path/line order.
pub fn detect_integrity(files: &[FileEvidence]) -> Vec<IntegritySignal> {
    let mut signals = markers::detect(files);
    signals.extend(accounting::detect(files));
    signals.extend(suppressions::detect(files));
    signals.sort_by(|left, right| left.path.cmp(&right.path).then(left.line.cmp(&right.line)));
    signals
}

fn scan(source: &ReviewSource, tables: &IntegrityTables, test_scope: bool) -> Option<Scan> {
    let tree = source.tree()?;
    let mut found = Scan::default();
    walk(
        tree.root_node(),
        source.content(),
        tables,
        test_scope,
        &mut found,
    );
    Some(found)
}

fn walk(
    node: Node<'_>,
    content: &str,
    tables: &IntegrityTables,
    test_scope: bool,
    found: &mut Scan,
) {
    if test_scope {
        if let Some(marker) = (tables.test_marker)(node, content) {
            found.markers.push(marker);
        }
        if let Some(name) = (tables.test_case)(node, content) {
            found
                .tests
                .push(accounting::TestFacts::of(node, content, tables, name));
        }
    }
    if let Some(label) = (tables.suppression)(node, content) {
        found
            .suppressions
            .push((label, node.start_position().row + 1));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, content, tables, test_scope, found);
    }
}

/// Fixture, `testdata`, and snapshot directories hold inputs a test reads, not
/// tests the runner executes; a marker there changes nothing that runs.
fn is_test_input_data(path: &std::path::Path) -> bool {
    const INPUT_DIRECTORIES: &[&str] = &[
        "fixtures",
        "__fixtures__",
        "testdata",
        "snapshots",
        "__snapshots__",
    ];
    let mut components = path.components();
    components.next_back(); // the file name itself is not a directory
    components.any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|name| INPUT_DIRECTORIES.contains(&name.to_ascii_lowercase().as_str()))
    })
}
