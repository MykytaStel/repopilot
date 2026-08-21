use super::{execute_check, run_checks};
use crate::config::loader::parse_config;
use crate::scan::session::WorkspaceRevision;
use crate::verification::{CancellationToken, VerificationStatus, select_checks};
use tempfile::tempdir;

fn check(root: &std::path::Path, toml: &str, id: &str) -> crate::verification::ValidatedCheck {
    let config = parse_config(toml, None).expect("valid config");
    select_checks(root, &config.verification.checks, &[id.into()])
        .expect("valid selection")
        .remove(0)
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
