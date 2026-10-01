use super::detect_gate_relaxation;
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
use crate::review::signals::content::ReviewSource;
use std::path::PathBuf;

fn relaxed(path: &str, before: &str, after: &str) -> Vec<String> {
    let file = ChangedFile {
        path: PathBuf::from(path),
        status: ChangeStatus::Modified,
        ranges: vec![ChangedRange { start: 3, end: 3 }],
        hunks: Vec::new(),
    };
    let pre = ReviewSource::new(before.to_string(), None);
    let post = ReviewSource::new(after.to_string(), None);
    detect_gate_relaxation(&file, Some(&pre), Some(&post))
        .into_iter()
        .map(|signal| signal.detail)
        .collect()
}

fn deleted(path: &str, before: &str) -> Vec<String> {
    let file = ChangedFile {
        path: PathBuf::from(path),
        status: ChangeStatus::Deleted,
        ranges: Vec::new(),
        hunks: Vec::new(),
    };
    let pre = ReviewSource::new(before.to_string(), None);
    detect_gate_relaxation(&file, Some(&pre), None)
        .into_iter()
        .map(|signal| signal.detail)
        .collect()
}

#[test]
fn a_deleted_workflow_removes_its_check_jobs() {
    assert_eq!(
        deleted(".github/workflows/ci.yml", WORKFLOW),
        vec!["job `test` that ran checks was removed"]
    );
    assert!(
        deleted(
            ".github/workflows/release.yml",
            "jobs:\n  publish:\n    steps:\n      - run: npm publish\n"
        )
        .is_empty()
    );
}

const WORKFLOW: &str = r#"name: ci
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Unit tests
        run: npm test
      - name: Lint
        run: npm run lint
  docs:
    runs-on: ubuntu-latest
    steps:
      - run: npm run docs
"#;

#[test]
fn github_actions_relaxations_are_reported() {
    let continue_on_error = WORKFLOW.replace(
        "        run: npm test\n",
        "        run: npm test\n        continue-on-error: true\n",
    );
    assert_eq!(
        relaxed(".github/workflows/ci.yml", WORKFLOW, &continue_on_error),
        vec!["check step `Unit tests` in job `test` now has `continue-on-error: true`"]
    );

    let swallowed = WORKFLOW.replace("run: npm run lint", "run: npm run lint || true");
    assert_eq!(
        relaxed(".github/workflows/ci.yml", WORKFLOW, &swallowed),
        vec!["check step `Lint` in job `test` now ends with `|| true`"]
    );

    let removed = WORKFLOW.replace("      - name: Lint\n        run: npm run lint\n", "");
    assert_eq!(
        relaxed(".github/workflows/ci.yml", WORKFLOW, &removed),
        vec!["check step `Lint` in job `test` was removed"]
    );

    let disabled = WORKFLOW.replace(
        "  test:\n    runs-on",
        "  test:\n    if: false\n    runs-on",
    );
    assert_eq!(
        relaxed(".github/workflows/ci.yml", WORKFLOW, &disabled),
        vec!["job `test` is now disabled with `if: false`"]
    );
}

#[test]
fn github_actions_changes_that_keep_checks_are_quiet() {
    let renamed_non_check = WORKFLOW.replace("run: npm run docs", "run: npm run docs:build");
    assert!(relaxed(".github/workflows/ci.yml", WORKFLOW, &renamed_non_check).is_empty());
    let pinned = WORKFLOW.replace("actions/checkout@v4", "actions/checkout@v5");
    assert!(relaxed(".github/workflows/ci.yml", WORKFLOW, &pinned).is_empty());
    assert!(relaxed(".github/workflows/ci.yml", WORKFLOW, "not: [valid").is_empty());
}

#[test]
fn gitlab_jobs_that_may_fail_or_vanish_are_reported() {
    let before = "stages: [test]\nunit:\n  stage: test\n  script:\n    - pytest\n";
    let allowed =
        "stages: [test]\nunit:\n  stage: test\n  allow_failure: true\n  script:\n    - pytest\n";
    assert_eq!(
        relaxed(".gitlab-ci.yml", before, allowed),
        vec!["job `unit` now has `allow_failure: true`"]
    );
    assert_eq!(
        relaxed(".gitlab-ci.yml", before, "stages: [test]\n"),
        vec!["job `unit` that ran checks was removed"]
    );
}

#[test]
fn npm_check_scripts_and_thresholds_are_reported() {
    let before = r#"{"scripts":{"test":"vitest run","lint":"eslint .","build":"tsc -b"},
      "jest":{"coverageThreshold":{"global":{"lines":90}}}}"#;
    let after = r#"{"scripts":{"test":"vitest run --passWithNoTests","build":"tsc -b"},
      "jest":{"coverageThreshold":{"global":{"lines":60}}}}"#;
    let mut found = relaxed("package.json", before, after);
    found.sort();
    assert_eq!(
        found,
        vec![
            "`jest.coverageThreshold.global.lines` lowered from 90 to 60",
            "check script `lint` was removed",
            "check script `test` now ends with `--passWithNoTests`",
        ]
    );
    let noop = before.replace("\"vitest run\"", "\"echo skipped\"");
    assert_eq!(
        relaxed("package.json", before, &noop),
        vec!["check script `test` no longer runs a check (`echo skipped`)"]
    );
}

#[test]
fn tsconfig_strictness_turned_off_is_reported_through_comments() {
    let before = "{\n  // shared\n  \"compilerOptions\": { \"strict\": true, \"noImplicitAny\": true, },\n}\n";
    let after = "{\n  // shared\n  \"compilerOptions\": { \"strict\": false, \"noImplicitAny\": true, },\n}\n";
    assert_eq!(
        relaxed("tsconfig.json", before, after),
        vec!["`compilerOptions.strict` turned off"]
    );
}

#[test]
fn python_tool_settings_are_reported() {
    let before = "[tool.pytest.ini_options]\naddopts = \"-q\"\n\n[tool.coverage.report]\nfail_under = 85\n\n[tool.mypy]\nstrict = true\n\n[tool.ruff.lint]\nignore = [\"E501\"]\n";
    let after = "[tool.pytest.ini_options]\naddopts = \"-q --deselect tests/test_api.py::test_retry\"\n\n[tool.coverage.report]\nfail_under = 70\n\n[tool.mypy]\nstrict = false\n\n[tool.ruff.lint]\nignore = [\"E501\", \"F401\"]\n";
    let mut found = relaxed("pyproject.toml", before, after);
    found.sort();
    assert_eq!(
        found,
        vec![
            "`tool.coverage.report.fail_under` lowered from 85 to 70",
            "`tool.mypy.strict` turned off",
            "`tool.pytest.ini_options.addopts` now passes `--deselect`; some tests no longer run",
            "`tool.ruff.lint.ignore` now ignores F401",
        ]
    );
    let raised = before.replace("fail_under = 85", "fail_under = 90");
    assert!(relaxed("pyproject.toml", before, &raised).is_empty());
}

#[test]
fn ini_and_codecov_settings_are_reported() {
    assert_eq!(
        relaxed(
            "setup.cfg",
            "[tool:pytest]\naddopts = -q\n",
            "[tool:pytest]\naddopts = -q\n    -m \"not slow\"\n"
        ),
        vec!["`tool:pytest.addopts` now passes `-m not`; some tests no longer run"]
    );
    assert!(
        relaxed(
            "setup.cfg",
            "[tool:pytest]\naddopts = -q\n",
            "[tool:pytest]\naddopts = -q\n    -m integration\n"
        )
        .is_empty(),
        "selecting a marker is not a negation"
    );
    assert_eq!(
        relaxed(
            ".coveragerc",
            "[report]\nfail_under = 80\n",
            "[report]\nfail_under = 50\n"
        ),
        vec!["`report.fail_under` lowered from 80 to 50"]
    );
    let before = "coverage:\n  status:\n    project:\n      default:\n        target: 80%\n";
    let after = "coverage:\n  status:\n    project:\n      default:\n        target: 60%\n        informational: true\n";
    let mut found = relaxed("codecov.yml", before, after);
    found.sort();
    assert_eq!(
        found,
        vec![
            "`coverage.status.project.default.informational` is now true; coverage no longer fails the check",
            "`coverage.status.project.default.target` lowered from 80 to 60",
        ]
    );
}
