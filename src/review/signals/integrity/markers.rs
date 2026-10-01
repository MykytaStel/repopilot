//! Skip and focus markers added by a change, net of moves between files.

use super::{FileEvidence, IntegrityKind, IntegritySignal};
use crate::review::signals::tables::{TestMarker, TestMarkerKind};
use std::collections::BTreeMap;

/// Reports markers added by the change, net of moves between changed files.
pub(super) fn detect(files: &[FileEvidence]) -> Vec<IntegritySignal> {
    let mut moved: BTreeMap<Identity, usize> = BTreeMap::new();
    for file in files {
        let post = counts(file.post.markers.iter());
        for (identity, count) in counts(file.pre.markers.iter()) {
            let remaining = count.saturating_sub(post.get(&identity).copied().unwrap_or(0));
            if remaining > 0 {
                *moved.entry(identity).or_default() += remaining;
            }
        }
    }

    let mut signals = Vec::new();
    for file in files {
        let pre = counts(file.pre.markers.iter());
        for (identity, count) in counts(file.post.markers.iter()) {
            let mut added = count.saturating_sub(pre.get(&identity).copied().unwrap_or(0));
            if let Some(pool) = moved.get_mut(&identity) {
                let matched = (*pool).min(added);
                *pool -= matched;
                added -= matched;
            }
            if added == 0 || (identity.0 == TestMarkerKind::Skip && !file.existed_before) {
                continue;
            }
            let mut candidates: Vec<(&TestMarker, bool)> = file
                .post
                .markers
                .iter()
                .zip(file.post_marker_in_diff.iter().copied())
                .filter(|(marker, _)| identity_of(marker) == identity)
                .collect();
            candidates.sort_by_key(|(marker, in_diff)| (!in_diff, marker.line));
            for (marker, _) in candidates.into_iter().take(added) {
                signals.push(signal(&file.path, marker));
            }
        }
    }
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
