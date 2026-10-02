//! Check thresholds passed on the command line, compared between the old and
//! new text of the same script, CI step, or tool setting: coverage floors
//! (`--cov-fail-under=90`, Node's `--test-coverage-lines=90`, c8/nyc
//! `--lines 90`, Vitest `--coverage.thresholds.lines=90`, `cargo llvm-cov
//! --fail-under-lines 90`) and lint warning caps (`eslint --max-warnings 0`).

use super::Relaxation;
use super::parse::number;

/// Whether a larger number makes the check stricter (a floor) or looser (a cap).
#[derive(Clone, Copy, PartialEq)]
enum Bound {
    Floor,
    Cap,
}

const FLAGS: &[(&str, Bound)] = &[
    ("--cov-fail-under", Bound::Floor),
    ("--fail-under", Bound::Floor),
    ("--fail-under-lines", Bound::Floor),
    ("--fail-under-regions", Bound::Floor),
    ("--fail-under-functions", Bound::Floor),
    ("--test-coverage-lines", Bound::Floor),
    ("--test-coverage-branches", Bound::Floor),
    ("--test-coverage-functions", Bound::Floor),
    ("--coverage.thresholds.lines", Bound::Floor),
    ("--coverage.thresholds.branches", Bound::Floor),
    ("--coverage.thresholds.functions", Bound::Floor),
    ("--coverage.thresholds.statements", Bound::Floor),
    ("--max-warnings", Bound::Cap),
];

/// c8 and nyc floors; these flag names are too generic to trust elsewhere.
const C8_FLAGS: &[&str] = &["--lines", "--branches", "--functions", "--statements"];

/// Thresholds that `new` relaxes compared with `old`. `subject` names where
/// the command lives, such as "check script `test`".
pub(super) fn relaxed_thresholds(old: &str, new: &str, subject: &str) -> Vec<Relaxation> {
    let (old_words, new_words) = (words(old), words(new));
    let coverage_tool = |words: &[String]| words.iter().any(|w| w == "c8" || w == "nyc");
    let mut flags: Vec<(&str, Bound)> = FLAGS.to_vec();
    if coverage_tool(&old_words) && coverage_tool(&new_words) {
        flags.extend(C8_FLAGS.iter().map(|flag| (*flag, Bound::Floor)));
    }
    let mut found = Vec::new();
    for (flag, bound) in flags {
        let Some(before) = strictest(&old_words, flag, bound) else {
            continue;
        };
        match strictest(&new_words, flag, bound) {
            Some(after) if looser(after, before, bound) => found.push(Relaxation::at(
                flag,
                format!(
                    "{subject} {} `{flag}` from {before} to {after}",
                    verb(bound)
                ),
            )),
            Some(_) => {}
            None => found.push(Relaxation::removed(format!(
                "{subject} no longer passes `{flag} {before}`"
            ))),
        }
    }
    found
}

fn verb(bound: Bound) -> &'static str {
    match bound {
        Bound::Floor => "lowers",
        Bound::Cap => "raises",
    }
}

fn looser(after: f64, before: f64, bound: Bound) -> bool {
    match bound {
        Bound::Floor => after < before,
        Bound::Cap => after > before,
    }
}

/// Shell words with quotes dropped, so `--flag="90"` and `--flag 90` read alike.
fn words(command: &str) -> Vec<String> {
    command
        .replace(['"', '\''], "")
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// The strictest value given for `flag`: the highest floor or the lowest cap.
fn strictest(words: &[String], flag: &str, bound: Bound) -> Option<f64> {
    let prefix = format!("{flag}=");
    let values = words.iter().enumerate().filter_map(|(index, word)| {
        if word == flag {
            words.get(index + 1).and_then(|next| number(next))
        } else {
            word.strip_prefix(&prefix).and_then(number)
        }
    });
    match bound {
        Bound::Floor => values.reduce(f64::max),
        Bound::Cap => values.reduce(f64::min),
    }
}

#[cfg(test)]
mod tests {
    use super::relaxed_thresholds;

    fn relaxed(old: &str, new: &str) -> Vec<String> {
        relaxed_thresholds(old, new, "check script `test`")
            .into_iter()
            .map(|relaxation| relaxation.detail)
            .collect()
    }

    #[test]
    fn lowered_and_dropped_floors_are_reported() {
        let node = "node --test --experimental-test-coverage --test-coverage-lines=90 test/";
        assert_eq!(
            relaxed(node, &node.replace("=90", "=75")),
            vec!["check script `test` lowers `--test-coverage-lines` from 90 to 75"]
        );
        assert_eq!(
            relaxed("pytest --cov=app --cov-fail-under 85", "pytest --cov=app"),
            vec!["check script `test` no longer passes `--cov-fail-under 85`"]
        );
        assert_eq!(
            relaxed(
                "c8 --check-coverage --lines 95 npm test",
                "c8 --check-coverage --lines 80 npm test"
            ),
            vec!["check script `test` lowers `--lines` from 95 to 80"]
        );
        assert_eq!(
            relaxed("eslint . --max-warnings 0", "eslint . --max-warnings=25"),
            vec!["check script `test` raises `--max-warnings` from 0 to 25"]
        );
    }

    #[test]
    fn stricter_unchanged_and_unrelated_flags_are_quiet() {
        assert!(relaxed("pytest --cov-fail-under=80", "pytest --cov-fail-under=90").is_empty());
        assert!(
            relaxed(
                "vitest --coverage.thresholds.lines=90",
                "vitest run --coverage.thresholds.lines=90"
            )
            .is_empty()
        );
        assert!(relaxed("eslint --max-warnings 10", "eslint --max-warnings 0").is_empty());
        // `--fail-under-lines` is its own flag, not `--fail-under`.
        assert!(
            relaxed(
                "cargo llvm-cov --fail-under-lines 80",
                "cargo llvm-cov --fail-under-lines 80"
            )
            .is_empty()
        );
        // Generic names count only under c8 or nyc.
        assert!(
            relaxed(
                "tail --lines 50 log.txt && npm test",
                "tail --lines 5 log.txt && npm test"
            )
            .is_empty()
        );
    }
}
