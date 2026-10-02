//! Relaxed check gates: CI and tool configuration that lets a failing check
//! pass (`continue-on-error`, `|| true`, a removed test step, a lowered
//! coverage threshold, strict type checking turned off).
//!
//! Configuration is parsed (YAML, JSON/JSONC, TOML, INI), never grepped, and
//! the pre- and post-change documents are compared key by key. A file that
//! cannot be parsed on either side yields nothing.

mod ci;
mod config;
mod flags;
mod parse;

use super::{IntegrityKind, IntegritySignal};
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::signals::content::ReviewSource;

/// One relaxation found in a configuration file.
pub(super) struct Relaxation {
    /// Text to locate the line in the post-change file; `None` points at the
    /// first changed line (for something that was removed).
    pub(super) anchor: Option<String>,
    pub(super) detail: String,
}

impl Relaxation {
    pub(super) fn at(anchor: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            anchor: Some(anchor.into()),
            detail: detail.into(),
        }
    }

    pub(super) fn removed(detail: impl Into<String>) -> Self {
        Self {
            anchor: None,
            detail: detail.into(),
        }
    }
}

/// Gate relaxations in one changed configuration file.
pub fn detect_gate_relaxation(
    file: &ChangedFile,
    pre: Option<&ReviewSource>,
    post: Option<&ReviewSource>,
) -> Vec<IntegritySignal> {
    let Some(pre) = pre else {
        return Vec::new();
    };
    let path = file.path_string();
    let name = path
        .rsplit('/')
        .next()
        .unwrap_or(&path)
        .to_ascii_lowercase();
    let is_gitlab = name == ".gitlab-ci.yml" || name == ".gitlab-ci.yaml";
    // A deleted pipeline file removes every check job it ran.
    let after = match post {
        Some(post) => post.content(),
        None if file.status == ChangeStatus::Deleted && is_github_workflow(&path) => "jobs: {}\n",
        None if file.status == ChangeStatus::Deleted && is_gitlab => "{}\n",
        None => return Vec::new(),
    };
    let before = pre.content();
    let found = if is_github_workflow(&path) {
        ci::github_actions(before, after)
    } else if is_gitlab {
        ci::gitlab(before, after)
    } else if name == "package.json" {
        config::package_json(before, after)
    } else if name.starts_with("tsconfig") && name.ends_with(".json") {
        config::tsconfig(before, after)
    } else if name == "pyproject.toml" {
        config::pyproject(before, after)
    } else if matches!(
        name.as_str(),
        "setup.cfg" | "pytest.ini" | "tox.ini" | ".coveragerc" | "mypy.ini"
    ) {
        config::ini(before, after)
    } else if name == "codecov.yml" || name == "codecov.yaml" || name == ".codecov.yml" {
        config::codecov(before, after)
    } else {
        Vec::new()
    };
    let first_changed = file.ranges.first().map_or(1, |range| range.start.max(1));
    found
        .into_iter()
        .map(|relaxation| IntegritySignal {
            kind: IntegrityKind::GateRelaxed,
            path: path.clone(),
            line: relaxation
                .anchor
                .as_deref()
                .and_then(|anchor| line_of(after, anchor))
                .unwrap_or(first_changed),
            detail: relaxation.detail,
        })
        .collect()
}

fn is_github_workflow(path: &str) -> bool {
    path.contains(".github/workflows/") && (path.ends_with(".yml") || path.ends_with(".yaml"))
}

/// 1-indexed line of the first occurrence of `needle`.
fn line_of(content: &str, needle: &str) -> Option<usize> {
    content
        .lines()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
}

/// Whether a command, step, job, or script name runs a check: tests, lint,
/// type checking, or coverage.
pub(super) fn runs_check(text: &str) -> bool {
    const CHECK_WORDS: &[&str] = &[
        "test",
        "tests",
        "pytest",
        "jest",
        "vitest",
        "mocha",
        "playwright",
        "cypress",
        "lint",
        "eslint",
        "ruff",
        "flake8",
        "pylint",
        "mypy",
        "pyright",
        "tsc",
        "typecheck",
        "clippy",
        "vet",
        "golangci",
        "check",
        "coverage",
        "spec",
        "tox",
        "nox",
        "rspec",
        "phpunit",
    ];
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| CHECK_WORDS.contains(&word))
}

/// Suffixes and flags that make a failing command exit 0.
pub(super) fn swallows_failure(command: &str) -> Option<&'static str> {
    const SWALLOWS: &[&str] = &[
        "|| true",
        "|| exit 0",
        "; exit 0",
        "|| :",
        "--passWithNoTests",
        "--exit-zero",
        "--exitzero",
        "--no-fail-on-error",
    ];
    let compact: String = command.split_whitespace().collect::<Vec<_>>().join(" ");
    SWALLOWS
        .iter()
        .copied()
        .find(|marker| compact.contains(marker))
}

#[cfg(test)]
mod tests;
