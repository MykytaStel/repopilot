//! Suppressions of RepoPilot itself: entries a change adds to
//! `.repopilot/overlay.toml`.
//!
//! The overlay is how a team acknowledges a signal on purpose, and `.repopilot/`
//! is left out of the changed-file list as RepoPilot's own state. Without this
//! check, a change could silence the review that judges it and nothing would
//! show it. Each new entry is reported with what it silences and why.

use super::{IntegrityKind, IntegritySignal};
use crate::knowledge::overlay::{OVERLAY_PATH, OverlayEntry, OverlayTarget, parse_overlay_content};
use crate::review::diff::{DiffTarget, git_show};
use std::path::{Path, PathBuf};

/// Overlay entries the change adds, as integrity signals.
pub fn detect_review_suppressions(
    repo_root: &Path,
    target: DiffTarget<'_>,
) -> Vec<IntegritySignal> {
    let base = match target {
        DiffTarget::WorkingTree => "HEAD",
        DiffTarget::Refs { base, .. } | DiffTarget::SinceRef { base } => base,
    };
    let before = git_show(repo_root, base, OVERLAY_PATH);
    let after = match target {
        DiffTarget::Refs { head, .. } => git_show(repo_root, head, OVERLAY_PATH),
        DiffTarget::WorkingTree | DiffTarget::SinceRef { .. } => {
            std::fs::read_to_string(repo_root.join(OVERLAY_PATH)).ok()
        }
    };
    match after {
        Some(after) => added_entries(before.as_deref(), &after),
        None => Vec::new(),
    }
}

/// Entries in `after` that `before` does not have, matched by target and path.
pub(super) fn added_entries(before: Option<&str>, after: &str) -> Vec<IntegritySignal> {
    let parse = |content: &str| parse_overlay_content(content, PathBuf::from(OVERLAY_PATH)).entries;
    let mut previous: Vec<(String, Option<String>)> = before
        .map(parse)
        .unwrap_or_default()
        .iter()
        .map(key)
        .collect();
    parse(after)
        .iter()
        .filter(|entry| {
            let entry_key = key(entry);
            match previous.iter().position(|existing| *existing == entry_key) {
                Some(index) => {
                    previous.remove(index);
                    false
                }
                None => true,
            }
        })
        .map(|entry| signal(entry, after))
        .collect()
}

fn key(entry: &OverlayEntry) -> (String, Option<String>) {
    (target_text(&entry.target), entry.path_text.clone())
}

fn target_text(target: &OverlayTarget) -> String {
    match target {
        OverlayTarget::Kind(kind) => format!("kind `{kind}`"),
        OverlayTarget::Rule(rule) => format!("rule `{rule}`"),
    }
}

fn signal(entry: &OverlayEntry, after: &str) -> IntegritySignal {
    let name = match &entry.target {
        OverlayTarget::Kind(kind) | OverlayTarget::Rule(kind) => kind.as_str(),
    };
    let effect = match entry.severity {
        Some(severity) => format!("sets {} to {severity:?}", target_text(&entry.target)),
        None => format!("suppresses {}", target_text(&entry.target)),
    };
    let scope = entry.path_text.as_deref().map_or_else(
        || " everywhere".to_string(),
        |path| format!(" for `{path}`"),
    );
    let reason = entry
        .reason
        .as_deref()
        .map(|reason| format!(" (reason: \"{reason}\")"))
        .unwrap_or_default();
    IntegritySignal {
        kind: IntegrityKind::ReviewSuppressionAdded,
        path: OVERLAY_PATH.to_string(),
        line: after
            .lines()
            .position(|line| line.contains(name))
            .map_or(1, |index| index + 1),
        detail: format!("new overlay entry {effect}{scope}{reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::added_entries;

    const BEFORE: &str = "[[overlay]]\nkind = \"behavioral.network-call-added\"\npath = \"src/gen/**\"\nreason = \"generated client\"\n";

    #[test]
    fn a_new_suppression_is_reported_with_scope_and_reason() {
        let after = format!(
            "{BEFORE}\n[[overlay]]\nkind = \"integrity.test-skipped\"\npath = \"tests/**\"\nreason = \"flaky upstream\"\n"
        );
        let signals = added_entries(Some(BEFORE), &after);
        assert_eq!(signals.len(), 1);
        assert_eq!(
            signals[0].detail,
            "new overlay entry suppresses kind `integrity.test-skipped` for `tests/**` (reason: \"flaky upstream\")"
        );
        assert_eq!(signals[0].line, 7);
    }

    #[test]
    fn unchanged_or_removed_entries_are_quiet_and_a_new_file_counts() {
        assert!(added_entries(Some(BEFORE), BEFORE).is_empty());
        assert!(added_entries(Some(BEFORE), "").is_empty());
        let rule = "[[overlay]]\nrule = \"architecture.large-file\"\n";
        let signals = added_entries(None, rule);
        assert_eq!(
            signals[0].detail,
            "new overlay entry suppresses rule `architecture.large-file` everywhere"
        );
    }
}
