//! Test-level accounting: test cases a change removed, and tests that kept
//! their name but lost assertions.
//!
//! Tests are identified by their qualified name (`header > renders title`,
//! `TestLogout.test_logout`, `tests::adds`). A name that leaves one changed
//! file and appears in another is a move. A test that keeps its name is
//! compared by assertion count; constant-only assertions (`assert True`,
//! `expect(true).toBe(true)`) never count, so trivializing an assertion reads
//! as removing it. A removed test whose body closely matches a new test in the
//! same file was renamed: it is compared by assertion count, not reported as
//! removed.

use super::{FileEvidence, IntegrityKind, IntegritySignal};
use crate::review::signals::tables::IntegrityTables;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

const LISTED_NAMES: usize = 5;
/// Jaccard similarity of body tokens at which a removed and a new test count
/// as one renamed test.
const RENAME_SIMILARITY: f64 = 0.75;

/// One test case as it exists on one side of the change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TestFacts {
    name: String,
    line: usize,
    assertions: usize,
    /// Distinct identifier and literal tokens of the body, without the name.
    tokens: BTreeSet<String>,
}

impl TestFacts {
    pub(super) fn of(
        node: Node<'_>,
        content: &str,
        tables: &IntegrityTables,
        name: String,
    ) -> Self {
        let text = node.utf8_text(content.as_bytes()).unwrap_or_default();
        Self {
            line: node.start_position().row + 1,
            assertions: count_assertions(node, content, tables),
            tokens: body_tokens(text, &name),
            name,
        }
    }
}

/// Assertions inside one test, not counting nested test cases.
fn count_assertions(node: Node<'_>, content: &str, tables: &IntegrityTables) -> usize {
    let mut count = 0;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if (tables.test_case)(child, content).is_some() {
            continue;
        }
        if (tables.is_assertion)(child, content) {
            count += 1;
        }
        count += count_assertions(child, content, tables);
    }
    count
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
        let (renamed, removed) = pair_renames(file, &removed);
        for (before, after) in renamed {
            if after.assertions < before.assertions {
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
fn difference<'a>(left: &'a [TestFacts], right: &[TestFacts]) -> BTreeMap<&'a str, usize> {
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

type Renamed<'a> = Vec<(&'a TestFacts, &'a TestFacts)>;

/// Splits removed names into renames (a new test in the same file with a
/// closely matching body) and genuine removals.
fn pair_renames<'a>(file: &'a FileEvidence, removed: &[&'a str]) -> (Renamed<'a>, Vec<&'a str>) {
    let appeared = difference(&file.post.tests, &file.pre.tests);
    let mut candidates: Vec<&TestFacts> = file
        .post
        .tests
        .iter()
        .filter(|test| appeared.contains_key(test.name.as_str()))
        .collect();
    let (mut renamed, mut remaining) = (Vec::new(), Vec::new());
    for name in removed {
        let Some(before) = file.pre.tests.iter().find(|test| test.name == *name) else {
            remaining.push(*name);
            continue;
        };
        let best = candidates
            .iter()
            .enumerate()
            .map(|(index, after)| (index, similarity(&before.tokens, &after.tokens)))
            .filter(|(_, score)| *score >= RENAME_SIMILARITY)
            .max_by(|left, right| left.1.total_cmp(&right.1));
        match best {
            Some((index, _)) => renamed.push((before, candidates.remove(index))),
            None => remaining.push(*name),
        }
    }
    (renamed, remaining)
}

fn similarity(left: &BTreeSet<String>, right: &BTreeSet<String>) -> f64 {
    let union = left.union(right).count();
    if union == 0 {
        return 0.0;
    }
    left.intersection(right).count() as f64 / union as f64
}

/// Test-framework words every test shares; they would make any two short
/// tests look alike. Matchers (`toBe`, `toThrow`) carry meaning and stay.
const BOILERPLATE: &[&str] = &[
    "it", "test", "describe", "expect", "assert", "self", "def", "fn", "func", "function", "async",
    "await", "const", "let", "var", "return", "true", "false", "None", "nil", "null", "t",
    "require", "mut", "pub",
];

/// Identifier and literal tokens of a test body, minus framework boilerplate
/// and the words of its name.
fn body_tokens(text: &str, name: &str) -> BTreeSet<String> {
    let name_words: BTreeSet<&str> = name
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
        .collect();
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|token| {
            !token.is_empty() && !name_words.contains(token) && !BOILERPLATE.contains(token)
        })
        .map(str::to_string)
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
            (post.assertions < pre.assertions).then(|| IntegritySignal {
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
