use super::{
    VerificationExecutionEvent, execute_check, run_checks, run_checks_observed,
    run_checks_observed_cached,
};
use crate::config::loader::parse_config;
use crate::scan::session::WorkspaceRevision;
use crate::verification::{CancellationToken, VerificationStatus, select_checks};
use tempfile::tempdir;

#[cfg(unix)]
fn executable(root: &std::path::Path, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = root.join("tool.sh");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write executable");
    let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).expect("executable permissions");
    path
}

#[cfg(unix)]
fn marker_runs(path: &std::path::Path) -> usize {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .chars()
        .filter(|character| *character == 'x')
        .count()
}

fn check(root: &std::path::Path, toml: &str, id: &str) -> crate::verification::ValidatedCheck {
    let config = parse_config(toml, None).expect("valid config");
    select_checks(root, &config.verification.checks, &[id.into()])
        .expect("valid selection")
        .remove(0)
}

#[test]
fn observed_checks_emit_ordered_lifecycle() {
    let temp = tempdir().expect("temp dir");
    let config = parse_config(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "repopilot-missing-unit-program"
[[verification.checks]]
id = "lint"
role = "lint"
program = "repopilot-missing-lint-program"
"#,
        None,
    )
    .expect("valid config");
    let checks = select_checks(
        temp.path(),
        &config.verification.checks,
        &["unit".into(), "lint".into()],
    )
    .expect("valid selection");
    let mut events = Vec::new();

    let outcomes = run_checks_observed(
        &checks,
        &[],
        &WorkspaceRevision::capture(temp.path()),
        &CancellationToken::new(),
        &mut |event| events.push(event),
    );

    assert_eq!(outcomes.len(), 2);
    assert_eq!(
        events,
        vec![
            VerificationExecutionEvent::Started {
                check_id: "lint".into(),
                index: 1,
                total: 2,
            },
            VerificationExecutionEvent::Completed {
                check_id: "lint".into(),
                index: 1,
                total: 2,
                status: VerificationStatus::Unavailable,
            },
            VerificationExecutionEvent::Started {
                check_id: "unit".into(),
                index: 2,
                total: 2,
            },
            VerificationExecutionEvent::Completed {
                check_id: "unit".into(),
                index: 2,
                total: 2,
                status: VerificationStatus::Unavailable,
            },
        ]
    );
}

#[cfg(unix)]
#[test]
fn observer_cancellation_prevents_next_spawn() {
    let temp = tempdir().expect("temp dir");
    let config = parse_config(
        r#"[[verification.checks]]
id = "first"
role = "test"
program = "sh"
args = ["-c", "true"]
[[verification.checks]]
id = "second"
role = "lint"
program = "sh"
args = ["-c", "printf spawned > second.txt"]
"#,
        None,
    )
    .expect("valid config");
    let checks = select_checks(
        temp.path(),
        &config.verification.checks,
        &["first".into(), "second".into()],
    )
    .expect("valid selection");
    let cancellation = CancellationToken::new();
    let observer_token = cancellation.clone();

    let outcomes = run_checks_observed(
        &checks,
        &[],
        &WorkspaceRevision::capture(temp.path()),
        &cancellation,
        &mut |event| {
            if matches!(
                event,
                VerificationExecutionEvent::Completed { ref check_id, .. }
                    if check_id == "first"
            ) {
                observer_token.cancel();
            }
        },
    );

    assert_eq!(outcomes.len(), 1);
    assert!(!temp.path().join("second.txt").exists());
}

#[cfg(unix)]
#[test]
fn records_pass_failure_and_bounded_redacted_streams() {
    let temp = tempdir().expect("temp dir");
    let check = check(
        temp.path(),
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "sh"
args = ["-c", "printf '\\033[31mok\\033[0m\\ntoken=fake-secret\\n'; printf 'err' >&2; exit 7"]
max_output_bytes = 20
"#,
        "unit",
    );
    let outcome = execute_check(
        &check,
        &WorkspaceRevision::capture(temp.path()),
        &CancellationToken::new(),
    );
    assert_eq!(outcome.status, VerificationStatus::Failed);
    assert_eq!(outcome.exit_code, Some(7));
    assert!(outcome.stdout_excerpt.contains("ok"));
    assert!(!outcome.stdout_excerpt.contains("fake-secret"));
    assert_eq!(outcome.stderr_excerpt, "err");
    assert!(outcome.stdout_truncated);
}

#[test]
fn unavailable_program_is_structured_evidence() {
    let temp = tempdir().expect("temp dir");
    let check = check(
        temp.path(),
        "[[verification.checks]]\nid = \"missing\"\nrole = \"build\"\nprogram = \"repopilot-program-that-does-not-exist\"\n",
        "missing",
    );
    let outcome = execute_check(
        &check,
        &WorkspaceRevision::capture(temp.path()),
        &CancellationToken::new(),
    );
    assert_eq!(outcome.status, VerificationStatus::Unavailable);
    assert_eq!(outcome.exit_code, None);
    assert!(!outcome.stderr_excerpt.is_empty());
}

#[cfg(unix)]
#[test]
fn timeout_returns_without_leaving_the_child_running() {
    let temp = tempdir().expect("temp dir");
    let check = check(
        temp.path(),
        "[[verification.checks]]\nid = \"slow\"\nrole = \"test\"\nprogram = \"sh\"\nargs = [\"-c\", \"sleep 30\"]\ntimeout_seconds = 1\n",
        "slow",
    );
    let started = std::time::Instant::now();
    let outcome = execute_check(
        &check,
        &WorkspaceRevision::capture(temp.path()),
        &CancellationToken::new(),
    );
    assert_eq!(outcome.status, VerificationStatus::TimedOut);
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[cfg(unix)]
#[test]
fn cancellation_is_structured_and_bounded() {
    let temp = tempdir().expect("temp dir");
    let check = check(
        temp.path(),
        "[[verification.checks]]\nid = \"slow\"\nrole = \"test\"\nprogram = \"sh\"\nargs = [\"-c\", \"sleep 30\"]\n",
        "slow",
    );
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let outcome = execute_check(
        &check,
        &WorkspaceRevision::capture(temp.path()),
        &cancellation,
    );
    assert_eq!(outcome.status, VerificationStatus::Cancelled);
}

#[test]
fn reuse_provenance_is_additive_and_omitted_for_executed_outcomes() {
    let temp = tempdir().expect("temp dir");
    let check = check(
        temp.path(),
        "[[verification.checks]]\nid = \"missing\"\nrole = \"test\"\nprogram = \"repopilot-missing-program\"\n",
        "missing",
    );
    let mut outcome = execute_check(
        &check,
        &WorkspaceRevision::capture(temp.path()),
        &CancellationToken::new(),
    );

    let fresh = serde_json::to_value(&outcome).expect("serialize fresh outcome");
    assert!(fresh.get("reused").is_none());
    let decoded: crate::verification::VerificationOutcome =
        serde_json::from_value(fresh).expect("fresh outcome must round trip");
    assert_eq!(decoded, outcome);

    outcome.reused = true;
    let reused = serde_json::to_value(&outcome).expect("serialize reused outcome");
    assert_eq!(reused["reused"], true);
}

#[cfg(unix)]
#[test]
fn cache_enabled_pass_executes_once_then_reuses_with_stable_lifecycle() {
    let root = tempdir().expect("root");
    let evidence = tempdir().expect("external evidence");
    let marker = evidence.path().join("runs.txt");
    executable(root.path(), "printf x >> \"$1\"");
    let check = check(
        root.path(),
        &format!(
            "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"./tool.sh\"\nargs = [{}]\n[verification.checks.cache]\nenabled = true\n",
            toml_string(marker.to_string_lossy().as_ref())
        ),
        "unit",
    );
    let revision = WorkspaceRevision::capture(root.path());
    let cancellation = CancellationToken::new();

    let first = run_checks_observed_cached(
        std::slice::from_ref(&check),
        &[],
        &revision,
        &cancellation,
        Some(root.path()),
        &mut |_| {},
    );
    let mut events = Vec::new();
    let second = run_checks_observed_cached(
        &[check],
        &[],
        &revision,
        &cancellation,
        Some(root.path()),
        &mut |event| events.push(event),
    );

    assert_eq!(marker_runs(&marker), 1);
    assert!(!first[0].reused);
    assert!(second[0].reused);
    assert_eq!(
        events,
        [
            VerificationExecutionEvent::Started {
                check_id: "unit".into(),
                index: 1,
                total: 1,
            },
            VerificationExecutionEvent::Completed {
                check_id: "unit".into(),
                index: 1,
                total: 1,
                status: VerificationStatus::Passed,
            },
        ]
    );
}

#[cfg(unix)]
#[test]
fn disabled_failed_skipped_and_incompatible_checks_never_reuse() {
    let root = tempdir().expect("root");
    let evidence = tempdir().expect("external evidence");
    let marker = evidence.path().join("runs.txt");
    executable(root.path(), "printf x >> \"$1\"; exit \"$2\"");
    let revision = WorkspaceRevision::capture(root.path());

    for (name, cache, exit) in [("disabled", false, "0"), ("failed", true, "1")] {
        let check = check(
            root.path(),
            &format!(
                "[[verification.checks]]\nid = \"{name}\"\nrole = \"test\"\nprogram = \"./tool.sh\"\nargs = [{}, \"{exit}\"]\ncache = {{ enabled = {cache} }}\n",
                toml_string(marker.to_string_lossy().as_ref())
            ),
            name,
        );
        for _ in 0..2 {
            let outcome = run_checks_observed_cached(
                std::slice::from_ref(&check),
                &[],
                &revision,
                &CancellationToken::new(),
                Some(root.path()),
                &mut |_| {},
            );
            assert!(!outcome[0].reused);
        }
    }
    assert_eq!(marker_runs(&marker), 4);

    let skipped = check(
        root.path(),
        "[[verification.checks]]\nid = \"skipped\"\nrole = \"test\"\nprogram = \"./tool.sh\"\npaths = [\"src/**\"]\ncache = { enabled = true }\n",
        "skipped",
    );
    let outcome = run_checks_observed_cached(
        &[skipped],
        &["docs/readme.md".into()],
        &revision,
        &CancellationToken::new(),
        Some(root.path()),
        &mut |_| {},
    );
    assert_eq!(outcome[0].status, VerificationStatus::Skipped);

    let tracked = root.path().join("tracked.txt");
    std::fs::write(&tracked, "before").expect("tracked input");
    executable(
        root.path(),
        "printf after > tracked.txt; printf x >> \"$1\"",
    );
    let mutating_revision = WorkspaceRevision::capture(root.path());
    let mutating = check(
        root.path(),
        &format!(
            "[[verification.checks]]\nid = \"mutating\"\nrole = \"test\"\nprogram = \"./tool.sh\"\nargs = [{}]\ncache = {{ enabled = true }}\n",
            toml_string(marker.to_string_lossy().as_ref())
        ),
        "mutating",
    );
    for _ in 0..2 {
        let outcome = run_checks_observed_cached(
            std::slice::from_ref(&mutating),
            &[],
            &mutating_revision,
            &CancellationToken::new(),
            Some(root.path()),
            &mut |_| {},
        );
        assert!(!outcome[0].revision_compatible);
        assert!(!outcome[0].reused);
    }
}

#[cfg(unix)]
#[test]
fn cancellation_after_started_never_attaches_a_readable_hit() {
    let root = tempdir().expect("root");
    let evidence = tempdir().expect("external evidence");
    let marker = evidence.path().join("runs.txt");
    executable(root.path(), "printf x >> \"$1\"");
    let check = check(
        root.path(),
        &format!(
            "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"./tool.sh\"\nargs = [{}]\ncache = {{ enabled = true }}\n",
            toml_string(marker.to_string_lossy().as_ref())
        ),
        "unit",
    );
    let revision = WorkspaceRevision::capture(root.path());
    run_checks_observed_cached(
        std::slice::from_ref(&check),
        &[],
        &revision,
        &CancellationToken::new(),
        Some(root.path()),
        &mut |_| {},
    );
    let cancellation = CancellationToken::new();
    let observer_token = cancellation.clone();
    let outcome = run_checks_observed_cached(
        &[check],
        &[],
        &revision,
        &cancellation,
        Some(root.path()),
        &mut |event| {
            if matches!(event, VerificationExecutionEvent::Started { .. }) {
                observer_token.cancel();
            }
        },
    );

    assert_eq!(outcome[0].status, VerificationStatus::Cancelled);
    assert!(!outcome[0].reused);
    assert_eq!(marker_runs(&marker), 1);
}

#[cfg(unix)]
fn toml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(unix)]
#[test]
fn revision_change_skips_remaining_checks() {
    let temp = tempdir().expect("temp dir");
    std::fs::write(temp.path().join("tracked.txt"), "before").expect("seed file");
    let config = parse_config(
        r#"[[verification.checks]]
id = "mutate"
role = "test"
program = "sh"
args = ["-c", "printf after > tracked.txt"]
[[verification.checks]]
id = "second"
role = "lint"
program = "sh"
args = ["-c", "printf spawned > second.txt"]
"#,
        None,
    )
    .expect("valid config");
    let checks = select_checks(
        temp.path(),
        &config.verification.checks,
        &["second".into(), "mutate".into()],
    )
    .expect("valid selection");
    let revision = WorkspaceRevision::capture(temp.path());
    let outcomes = run_checks(
        &checks,
        &["tracked.txt".into()],
        &revision,
        &CancellationToken::new(),
    );
    assert_eq!(outcomes[0].check_id, "mutate");
    assert!(!outcomes[0].revision_compatible);
    assert_eq!(outcomes[1].status, VerificationStatus::Skipped);
    assert!(!temp.path().join("second.txt").exists());
}
