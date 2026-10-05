//! Test-level accounting: test cases a change removed, and tests that kept
//! their name but lost assertions.
//!
//! Tests are identified by their qualified name (`header > renders title`,
//! `TestLogout.test_logout`, `tests::adds`). A name that leaves one changed
//! file and appears in another is a move. A test that keeps its name is
//! compared by assertion count, including the same-file helpers it calls
//! (`helpers.rs`); constant-only assertions (`assert True`,
//! `expect(true).toBe(true)`) never count, so trivializing an assertion reads
//! as removing it. A removed test whose body closely matches a new test in the
//! same file was renamed: it is compared by assertion count, not reported as
//! removed.

mod renames;

use super::helpers::{self, Body};
use super::{FileEvidence, IntegrityKind, IntegritySignal};
use std::collections::BTreeMap;
use tree_sitter::Node;

const LISTED_NAMES: usize = 5;

/// One test case as it exists on one side of the change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TestFacts {
    name: String,
    line: usize,
    /// The test's own body, before helpers are folded in.
    body: Body,
    assertions: usize,
    looped: bool,
    /// The test's source; its tokens are computed only when a removed test
    /// is matched against new ones (`pair_renames`).
    text: String,
}

impl TestFacts {
    /// A test whose body the scan pass fills through `body_mut`.
    pub(super) fn of(node: Node<'_>, content: &str, name: String) -> Self {
        let text = node.utf8_text(content.as_bytes()).unwrap_or_default();
        Self {
            line: node.start_position().row + 1,
            body: Body::default(),
            assertions: 0,
            looped: false,
            text: text.to_string(),
            name,
        }
    }

    pub(super) fn body_mut(&mut self) -> &mut Body {
        &mut self.body
    }

    /// Folds in the assertions of the same-file helpers this test calls.
    pub(super) fn include_helpers(&mut self, helpers: &BTreeMap<String, Body>) {
        let resolved = helpers::resolve(&self.body, helpers);
        self.assertions = resolved.assertions;
        self.looped = resolved.looped;
    }

    /// Fewer assertions than `before`, unless the test became a loop over
    /// cases, where one statement can check what several did.
    fn checks_less_than(&self, before: &TestFacts) -> bool {
        self.assertions < before.assertions && (before.looped || !self.looped)
    }
}

pub(super) fn detect(files: &[FileEvidence]) -> Vec<IntegritySignal> {
    let mut arrived: BTreeMap<&str, usize> = BTreeMap::new();
    for file in files {
        for (name, count) in difference(&file.post.tests, &file.pre.tests) {
            *arrived.entry(name).or_default() += count;
        }
    }

    let mut signals = Vec::new();
    for file in files {
        if !file.existed_before || file.removal_reported_elsewhere {
            continue;
        }
        let mut removed = Vec::new();
        for (name, count) in difference(&file.pre.tests, &file.post.tests) {
            let moved = arrived.get_mut(name).map_or(0, |pool| {
                let matched = (*pool).min(count);
                *pool -= matched;
                matched
            });
            removed.extend(std::iter::repeat_n(name, count - moved));
        }
        let (renamed, removed) = renames::pair(file, &removed);
        for (before, after) in renamed {
            if after.checks_less_than(before) {
                signals.push(IntegritySignal {
                    kind: IntegrityKind::AssertionsRemoved,
                    path: file.path.clone(),
                    line: after.line,
                    detail: format!(
                        "\"{}\" renamed to \"{}\": assertions {} → {}",
                        before.name, after.name, before.assertions, after.assertions
                    ),
                });
            }
        }
        if !removed.is_empty() {
            signals.push(removed_signal(file, &removed));
        }
        signals.extend(assertion_signals(file));
    }
    signals
}

/// Names in `left` beyond their count in `right` (a multiset difference).
pub(super) fn difference<'a>(
    left: &'a [TestFacts],
    right: &[TestFacts],
) -> BTreeMap<&'a str, usize> {
    let mut counts: BTreeMap<&str, isize> = BTreeMap::new();
    for test in left {
        *counts.entry(test.name.as_str()).or_default() += 1;
    }
    for test in right {
        if let Some(count) = counts.get_mut(test.name.as_str()) {
            *count -= 1;
        }
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(name, count)| (name, count as usize))
        .collect()
}

fn removed_signal(file: &FileEvidence, removed: &[&str]) -> IntegritySignal {
    let mut detail = format!(
        "{} test{} removed: {}",
        removed.len(),
        plural(removed.len()),
        listed(removed.iter().copied())
    );
    let appeared = difference(&file.post.tests, &file.pre.tests);
    if !appeared.is_empty() {
        let count: usize = appeared.values().sum();
        detail.push_str(&format!(
            "; {count} new test{} in this file: {} — check whether it covers the same behavior",
            plural(count),
            listed(appeared.keys().copied())
        ));
    }
    IntegritySignal {
        kind: IntegrityKind::TestRemoved,
        path: file.path.clone(),
        line: file.first_changed_line,
        detail,
    }
}

/// Tests present once on both sides whose assertion count went down.
fn assertion_signals(file: &FileEvidence) -> Vec<IntegritySignal> {
    let before = unique_by_name(&file.pre.tests);
    let after = unique_by_name(&file.post.tests);
    before
        .iter()
        .filter_map(|(name, pre)| {
            let post = after.get(name)?;
            post.checks_less_than(pre).then(|| IntegritySignal {
                kind: IntegrityKind::AssertionsRemoved,
                path: file.path.clone(),
                line: post.line,
                detail: if post.assertions == 0 {
                    format!(
                        "\"{name}\": assertions {} → 0; the test can no longer fail on a wrong result",
                        pre.assertions
                    )
                } else {
                    format!("\"{name}\": assertions {} → {}", pre.assertions, post.assertions)
                },
            })
        })
        .collect()
}

/// Tests whose name occurs exactly once on one side of the change.
fn unique_by_name(tests: &[TestFacts]) -> BTreeMap<&str, &TestFacts> {
    let mut by_name: BTreeMap<&str, Vec<&TestFacts>> = BTreeMap::new();
    for test in tests {
        by_name.entry(test.name.as_str()).or_default().push(test);
    }
    by_name
        .into_iter()
        .filter_map(|(name, tests)| (tests.len() == 1).then(|| (name, tests[0])))
        .collect()
}

fn listed<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let names: Vec<&str> = names.collect();
    let mut shown: Vec<String> = names
        .iter()
        .take(LISTED_NAMES)
        .map(|name| format!("\"{name}\""))
        .collect();
    if names.len() > LISTED_NAMES {
        shown.push(format!("and {} more", names.len() - LISTED_NAMES));
    }
    shown.join(", ")
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
