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
mod acknowledgement;
mod gates;
mod helpers;
#[cfg(test)]
mod helpers_tests;
mod markers;
mod moves;
#[cfg(test)]
mod rename_tests;
mod scan;
mod suppressions;
#[cfg(test)]
mod suppressions_tests;
pub(crate) mod syntax;
#[cfg(test)]
mod tests;

use crate::languages::integrity_for_extension;
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::signals::content::ReviewSource;
pub use acknowledgement::detect_review_suppressions;
pub use gates::detect_gate_relaxation;
pub use moves::moved_test_files;
use serde::Serialize;

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
    ReviewSuppressionAdded,
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
    /// Pre-change text, to tell a skip in an existing test from one in new code.
    pre_text: String,
    pre: scan::Scan,
    post: scan::Scan,
    /// Post-change markers on changed lines, by index into `post.markers`.
    post_marker_in_diff: Vec<bool>,
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
    let (pre, pre_text) = if existed_before {
        let source = pre?;
        (
            scan::scan(source, tables, test_scope)?,
            source.content().to_string(),
        )
    } else {
        (scan::Scan::default(), String::new())
    };
    let post = match post {
        Some(source) => scan::scan(source, tables, test_scope)?,
        None if file.status == ChangeStatus::Deleted => scan::Scan::default(),
        None => return None,
    };
    Some(FileEvidence {
        path: file.path_string(),
        existed_before,
        removal_reported_elsewhere: is_test_file && post.tests.is_empty(),
        first_changed_line: file.ranges.first().map_or(1, |range| range.start.max(1)),
        pre_text,
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
