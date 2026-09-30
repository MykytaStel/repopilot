//! Test-integrity signals: a change that weakens the checks judging it.
//!
//! This covers skip and focus markers (`it.only`, `@pytest.mark.skip`,
//! `t.Skip`, `#[ignore]`). Markers are counted over the whole pre- and
//! post-change file, not only the changed lines, so re-indenting or moving a
//! marker within a file is not a new marker. A marker that disappears from one
//! changed file and appears in another is a move and is not reported either.
//! A skip in a brand-new file weakens nothing that ran before and is not
//! reported; a focus marker is, because it stops the rest of the suite.

pub(crate) mod syntax;
#[cfg(test)]
mod tests;

use crate::languages::integrity_for_extension;
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::signals::content::ReviewSource;
use crate::review::signals::tables::{IntegrityTables, TestMarker, TestMarkerKind};
use serde::Serialize;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// The category of test-integrity change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrityKind {
    TestFocused,
    TestSkipped,
}

/// A test-integrity signal detected in a changed file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegritySignal {
    pub kind: IntegrityKind,
    pub path: String,
    pub line: usize,
    pub detail: String,
}

/// The skip/focus markers of one changed file, before and after the change.
#[derive(Debug)]
pub struct FileMarkers {
    path: String,
    existed_before: bool,
    pre: Vec<TestMarker>,
    /// Post-change markers, each flagged when it sits on a changed line.
    post: Vec<(TestMarker, bool)>,
}

/// Collects a changed file's markers, or `None` when no recognizer applies or
/// a side that should exist cannot be parsed (never guess from half a picture).
pub fn collect_file_markers(
    file: &ChangedFile,
    pre: Option<&ReviewSource>,
    post: Option<&ReviewSource>,
) -> Option<FileMarkers> {
    let ext = file.path.extension().and_then(|ext| ext.to_str())?;
    let tables = integrity_for_extension(ext)?;
    if is_test_input_data(&file.path)
        || (!tables.applies_outside_test_files
            && !crate::audits::context::classify::helpers::is_test_file(&file.path))
    {
        return None;
    }
    let existed_before = !matches!(file.status, ChangeStatus::Added | ChangeStatus::Untracked);
    let pre_markers = if existed_before {
        markers(pre?, tables)?
    } else {
        Vec::new()
    };
    let post_markers = match post {
        Some(source) => markers(source, tables)?,
        None if file.status == ChangeStatus::Deleted => Vec::new(),
        None => return None,
    };
    Some(FileMarkers {
        path: file.path_string(),
        existed_before,
        pre: pre_markers,
        post: post_markers
            .into_iter()
            .map(|marker| {
                let in_diff = file.contains_line(marker.line);
                (marker, in_diff)
            })
            .collect(),
    })
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

/// Reports markers added by the change, net of moves between changed files.
pub fn detect_integrity(files: &[FileMarkers]) -> Vec<IntegritySignal> {
    let mut moved: BTreeMap<Identity, usize> = BTreeMap::new();
    for file in files {
        let post = counts(file.post.iter().map(|(marker, _)| marker));
        for (identity, count) in counts(file.pre.iter()) {
            let remaining = count.saturating_sub(post.get(&identity).copied().unwrap_or(0));
            if remaining > 0 {
                *moved.entry(identity).or_default() += remaining;
            }
        }
    }

    let mut signals = Vec::new();
    for file in files {
        let pre = counts(file.pre.iter());
        for (identity, count) in counts(file.post.iter().map(|(marker, _)| marker)) {
            let mut added = count.saturating_sub(pre.get(&identity).copied().unwrap_or(0));
            if let Some(pool) = moved.get_mut(&identity) {
                let matched = (*pool).min(added);
                *pool -= matched;
                added -= matched;
            }
            if added == 0 || (identity.0 == TestMarkerKind::Skip && !file.existed_before) {
                continue;
            }
            let mut candidates: Vec<&(TestMarker, bool)> = file
                .post
                .iter()
                .filter(|(marker, _)| identity_of(marker) == identity)
                .collect();
            candidates.sort_by_key(|(marker, in_diff)| (!in_diff, marker.line));
            for (marker, _) in candidates.into_iter().take(added) {
                signals.push(signal(&file.path, marker));
            }
        }
    }
    signals.sort_by(|left, right| left.path.cmp(&right.path).then(left.line.cmp(&right.line)));
    signals
}

type Identity = (TestMarkerKind, String, Option<String>);

fn identity_of(marker: &TestMarker) -> Identity {
    (marker.kind, marker.marker.clone(), marker.test_name.clone())
}

fn counts<'a>(markers: impl Iterator<Item = &'a TestMarker>) -> BTreeMap<Identity, usize> {
    let mut counts = BTreeMap::new();
    for marker in markers {
        *counts.entry(identity_of(marker)).or_default() += 1;
    }
    counts
}

fn markers(source: &ReviewSource, tables: &IntegrityTables) -> Option<Vec<TestMarker>> {
    let tree = source.tree()?;
    let mut found = Vec::new();
    walk(tree.root_node(), source.content(), tables, &mut found);
    Some(found)
}

fn walk(node: Node<'_>, content: &str, tables: &IntegrityTables, found: &mut Vec<TestMarker>) {
    if let Some(marker) = (tables.test_marker)(node, content) {
        found.push(marker);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, content, tables, found);
    }
}

fn signal(path: &str, marker: &TestMarker) -> IntegritySignal {
    let on = marker
        .test_name
        .as_deref()
        .map(|name| format!(" on \"{name}\""))
        .unwrap_or_default();
    let (kind, detail) = match marker.kind {
        TestMarkerKind::Focus => (
            IntegrityKind::TestFocused,
            format!(
                "`{}` added{on}; other tests in its scope stop running while it is committed",
                marker.marker
            ),
        ),
        TestMarkerKind::Skip => {
            let why = marker
                .reason
                .as_deref()
                .map(|reason| format!(" (reason: \"{reason}\")"))
                .unwrap_or_default();
            (
                IntegrityKind::TestSkipped,
                format!(
                    "`{}` added{on}{why}; its result no longer fails the run",
                    marker.marker
                ),
            )
        }
    };
    IntegritySignal {
        kind,
        path: path.to_string(),
        line: marker.line,
        detail,
    }
}
