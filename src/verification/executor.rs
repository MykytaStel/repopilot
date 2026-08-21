use crate::scan::session::WorkspaceRevision;
use crate::verification::model::{CancellationToken, VerificationOutcome, VerificationStatus};
use crate::verification::policy::{ValidatedCheck, ValidatedProgram};
use crate::verification::redaction::{CapturedStream, capture_and_redact};
use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

mod platform;
use platform::{ProcessTree, configure_process_tree};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const PORTABLE_ENV_KEYS: &[&str] = &[
    "PATH",
    "HOME",
    "USERPROFILE",
    "SYSTEMROOT",
    "TMPDIR",
    "TEMP",
    "TMP",
];

pub fn run_checks(
    checks: &[ValidatedCheck],
    evidence_paths: &[std::path::PathBuf],
    revision: &WorkspaceRevision,
    cancellation: &CancellationToken,
) -> Vec<VerificationOutcome> {
    let mut outcomes = Vec::with_capacity(checks.len());
    let mut revision_changed = false;
    for check in checks {
        if revision_changed {
            outcomes.push(skipped_outcome(
                check,
                revision,
                "workspace revision changed before this check could run",
            ));
            continue;
        }
        if !check.is_applicable(evidence_paths.iter().map(std::path::PathBuf::as_path)) {
            outcomes.push(skipped_outcome(
                check,
                revision,
                "no changed or impacted path matched this check",
            ));
            continue;
        }
        let outcome = execute_check(check, revision, cancellation);
        revision_changed = !outcome.revision_compatible;
        outcomes.push(outcome);
    }
    outcomes
}

pub fn execute_check(
    check: &ValidatedCheck,
    revision_before: &WorkspaceRevision,
    cancellation: &CancellationToken,
) -> VerificationOutcome {
    let started = Instant::now();
    let mut command = command_for(check);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return unavailable_outcome(check, revision_before, started, error.to_string());
        }
    };
    let process_tree = match ProcessTree::attach(&child) {
        Ok(tree) => tree,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return unavailable_outcome(check, revision_before, started, error);
        }
    };
    let stdout = spawn_reader(child.stdout.take(), check.max_output_bytes);
    let stderr = spawn_reader(child.stderr.take(), check.max_output_bytes);
    let deadline = started + Duration::from_secs(check.timeout_seconds);
    let mut forced_status = None;

    let exit = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if cancellation.is_cancelled() => {
                forced_status = Some(VerificationStatus::Cancelled);
                process_tree.terminate(&mut child);
                break child.wait().ok();
            }
            Ok(None) if Instant::now() >= deadline => {
                forced_status = Some(VerificationStatus::TimedOut);
                process_tree.terminate(&mut child);
                break child.wait().ok();
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(_) => {
                process_tree.terminate(&mut child);
                break child.wait().ok();
            }
        }
    };

    let stdout = join_reader(stdout);
    let stderr = join_reader(stderr);
    let revision_after = WorkspaceRevision::capture(&check.working_directory);
    let status = forced_status.unwrap_or_else(|| {
        if exit.is_some_and(|status| status.success()) {
            VerificationStatus::Passed
        } else {
            VerificationStatus::Failed
        }
    });
    VerificationOutcome {
        check_id: check.id().to_string(),
        role: check.role,
        status,
        duration_ms: duration_ms(started.elapsed()),
        exit_code: exit.and_then(|status| status.code()),
        working_directory: check.working_directory_label.clone(),
        stdout_excerpt: stdout.excerpt,
        stderr_excerpt: stderr.excerpt,
        stdout_truncated: stdout.truncated,
        stderr_truncated: stderr.truncated,
        revision_before: revision_before.id().to_string(),
        revision_after: revision_after.id().to_string(),
        revision_compatible: revision_before == &revision_after,
        limitations: Vec::new(),
    }
}

fn command_for(check: &ValidatedCheck) -> Command {
    let mut command = match &check.program {
        ValidatedProgram::Bare(program) => Command::new(program),
        ValidatedProgram::RepositoryRelative(program) => Command::new(program),
    };
    command
        .args(&check.args)
        .current_dir(&check.working_directory)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for key in PORTABLE_ENV_KEYS {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    configure_process_tree(&mut command);
    command
}

fn spawn_reader(
    stream: Option<impl Read + Send + 'static>,
    limit: usize,
) -> thread::JoinHandle<CapturedStream> {
    thread::spawn(move || {
        let Some(mut stream) = stream else {
            return capture_and_redact(&[], false);
        };
        let mut retained = Vec::with_capacity(limit.min(8_192));
        let mut buffer = [0_u8; 8_192];
        let mut observed_more = false;
        loop {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    let available = limit.saturating_sub(retained.len());
                    let keep = available.min(read);
                    retained.extend_from_slice(&buffer[..keep]);
                    observed_more |= keep < read;
                }
            }
        }
        capture_and_redact(&retained, observed_more)
    })
}

fn join_reader(reader: thread::JoinHandle<CapturedStream>) -> CapturedStream {
    reader
        .join()
        .unwrap_or_else(|_| capture_and_redact(&[], true))
}

fn unavailable_outcome(
    check: &ValidatedCheck,
    revision: &WorkspaceRevision,
    started: Instant,
    error: String,
) -> VerificationOutcome {
    let error = capture_and_redact(error.as_bytes(), false);
    VerificationOutcome {
        check_id: check.id().to_string(),
        role: check.role,
        status: VerificationStatus::Unavailable,
        duration_ms: duration_ms(started.elapsed()),
        exit_code: None,
        working_directory: check.working_directory_label.clone(),
        stdout_excerpt: String::new(),
        stderr_excerpt: error.excerpt,
        stdout_truncated: false,
        stderr_truncated: error.truncated,
        revision_before: revision.id().to_string(),
        revision_after: revision.id().to_string(),
        revision_compatible: true,
        limitations: vec!["configured program could not be started".to_string()],
    }
}

fn skipped_outcome(
    check: &ValidatedCheck,
    revision: &WorkspaceRevision,
    limitation: &str,
) -> VerificationOutcome {
    VerificationOutcome {
        check_id: check.id().to_string(),
        role: check.role,
        status: VerificationStatus::Skipped,
        duration_ms: 0,
        exit_code: None,
        working_directory: check.working_directory_label.clone(),
        stdout_excerpt: String::new(),
        stderr_excerpt: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        revision_before: revision.id().to_string(),
        revision_after: revision.id().to_string(),
        revision_compatible: true,
        limitations: vec![limitation.to_string()],
    }
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests;
