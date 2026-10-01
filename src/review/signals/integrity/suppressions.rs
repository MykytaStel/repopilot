//! Lint, type-check, and coverage suppressions a change adds.
//!
//! Counted by label (directive plus rules) over the whole file before and
//! after. A suppression that leaves one changed file and appears in another
//! moved with its code and is not new.

use super::{FileEvidence, IntegrityKind, IntegritySignal};
use std::collections::BTreeMap;

pub(super) fn detect(files: &[FileEvidence]) -> Vec<IntegritySignal> {
    let mut departed: BTreeMap<&str, usize> = BTreeMap::new();
    for file in files {
        let post = counts(&file.post.suppressions);
        for (label, count) in counts(&file.pre.suppressions) {
            let gone = count.saturating_sub(post.get(label).copied().unwrap_or(0));
            if gone > 0 {
                *departed.entry(label).or_default() += gone;
            }
        }
    }

    let mut signals = Vec::new();
    for file in files {
        let pre = counts(&file.pre.suppressions);
        for (label, after) in counts(&file.post.suppressions) {
            let before = pre.get(label).copied().unwrap_or(0);
            let mut added = after.saturating_sub(before);
            if let Some(pool) = departed.get_mut(label) {
                let matched = (*pool).min(added);
                *pool -= matched;
                added -= matched;
            }
            if added == 0 {
                continue;
            }
            let line = file
                .post
                .suppressions
                .iter()
                .filter(|(candidate, _)| candidate == label)
                .map(|(_, line)| *line)
                .max()
                .unwrap_or(file.first_changed_line);
            signals.push(IntegritySignal {
                kind: IntegrityKind::SuppressionAdded,
                path: file.path.clone(),
                line,
                detail: format!("`{label}` added ({before} → {after} in this file)"),
            });
        }
    }
    signals
}

fn counts(suppressions: &[(String, usize)]) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for (label, _) in suppressions {
        *counts.entry(label.as_str()).or_default() += 1;
    }
    counts
}
