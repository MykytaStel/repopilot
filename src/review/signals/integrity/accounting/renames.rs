//! Renamed tests: a test that left a file paired with a new test in the same
//! file. A pair is compared by assertion count instead of being reported as a
//! removed test.

use super::{FileEvidence, TestFacts, difference};
use std::collections::BTreeSet;

/// Jaccard similarity of body tokens at which a removed and a new test count
/// as one renamed test.
const RENAME_SIMILARITY: f64 = 0.75;
/// Fewest body tokens a new test needs to count as a reduced copy of a removed
/// one; a two-call test is contained in almost anything.
const MIN_REDUCED_TOKENS: usize = 4;

pub(super) type Renamed<'a> = Vec<(&'a TestFacts, &'a TestFacts)>;

/// Splits removed names into renames and genuine removals. A new test in the
/// same file is the renamed one when its body closely matches, or when its
/// body is part of the old one and checks less: a rename that dropped a check
/// is reported as that drop.
pub(super) fn pair<'a>(file: &'a FileEvidence, removed: &[&'a str]) -> (Renamed<'a>, Vec<&'a str>) {
    let appeared = difference(&file.post.tests, &file.pre.tests);
    let mut candidates: Vec<(&TestFacts, Tokens)> = file
        .post
        .tests
        .iter()
        .filter(|test| appeared.contains_key(test.name.as_str()))
        .map(|test| (test, Tokens::of(test)))
        .collect();
    let (mut renamed, mut remaining) = (Vec::new(), Vec::new());
    for name in removed {
        let Some(before) = file.pre.tests.iter().find(|test| test.name == *name) else {
            remaining.push(*name);
            continue;
        };
        let tokens = Tokens::of(before);
        let scored = candidates
            .iter()
            .enumerate()
            .map(|(index, (after, after_tokens))| (index, tokens.similarity(after_tokens), *after));
        let similar = scored
            .clone()
            .filter(|(_, score, _)| *score >= RENAME_SIMILARITY)
            .max_by(|left, right| left.1.total_cmp(&right.1));
        let reduced = || {
            scored
                .clone()
                .filter(|(index, _, after)| {
                    tokens.contains(&candidates[*index].1) && after.checks_less_than(before)
                })
                .max_by(|left, right| left.1.total_cmp(&right.1))
        };
        match similar.or_else(reduced) {
            Some((index, _, _)) => renamed.push((before, candidates.remove(index).0)),
            None => remaining.push(*name),
        }
    }
    (renamed, remaining)
}

/// A test's body and name as word sets, for rename matching.
struct Tokens {
    body: BTreeSet<String>,
    name: BTreeSet<String>,
}

impl Tokens {
    fn of(test: &TestFacts) -> Self {
        Self {
            body: words(&test.text)
                .filter(|token| !BOILERPLATE.contains(token))
                .map(str::to_string)
                .collect(),
            name: words(&test.name).map(str::to_string).collect(),
        }
    }

    /// The body without the words of the test's own name. Removing the other
    /// name's words too would pair a test that kept its setup but now checks
    /// other properties, a substitution.
    fn own_body(&self) -> BTreeSet<&String> {
        self.body
            .iter()
            .filter(|token| !self.name.contains(*token))
            .collect()
    }

    fn bodies<'a>(&'a self, other: &'a Tokens) -> (BTreeSet<&'a String>, BTreeSet<&'a String>) {
        (self.own_body(), other.own_body())
    }

    /// Jaccard similarity of the two bodies.
    fn similarity(&self, other: &Tokens) -> f64 {
        let (left, right) = self.bodies(other);
        let union = left.union(&right).count();
        if union == 0 {
            return 0.0;
        }
        left.intersection(&right).count() as f64 / union as f64
    }

    /// Whether `other`'s body is a non-trivial part of this one.
    fn contains(&self, other: &Tokens) -> bool {
        let (whole, part) = self.bodies(other);
        part.len() >= MIN_REDUCED_TOKENS && part.is_subset(&whole)
    }
}

/// Test-framework words every test shares; they would make any two short
/// tests look alike. Matchers (`toBe`, `toThrow`) carry meaning and stay.
const BOILERPLATE: &[&str] = &[
    "it", "test", "describe", "expect", "assert", "self", "def", "fn", "func", "function", "async",
    "await", "const", "let", "var", "return", "true", "false", "None", "nil", "null", "t",
    "require", "mut", "pub",
];

/// Identifier and literal tokens of a test's source or name.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
}
