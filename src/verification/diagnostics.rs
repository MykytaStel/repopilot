use super::model::{
    VerificationDiagnostic, VerificationDiagnosticKind, VerificationDiagnostics, VerificationStatus,
};
use super::policy::{ValidatedCheck, ValidatedProgram};
use std::path::{Path, PathBuf};

const ADAPTER: &str = "pytest-node-v1";

pub(crate) fn diagnostics_for_check(
    check: &ValidatedCheck,
    status: VerificationStatus,
    stdout: &str,
    stderr: &str,
    truncated: bool,
) -> Option<VerificationDiagnostics> {
    if !supports_pytest(check) {
        return None;
    }
    if truncated {
        return Some(unavailable("pytest output was truncated"));
    }
    if !matches!(
        status,
        VerificationStatus::Passed | VerificationStatus::Failed
    ) {
        return Some(unavailable(match status {
            VerificationStatus::TimedOut => "verification timed out before a complete diagnostic",
            VerificationStatus::Unavailable => "verification program was unavailable",
            VerificationStatus::Cancelled => {
                "verification was cancelled before a complete diagnostic"
            }
            VerificationStatus::Skipped => "verification was skipped before a complete diagnostic",
            VerificationStatus::Passed | VerificationStatus::Failed => unreachable!(),
        }));
    }
    if status == VerificationStatus::Passed {
        return Some(VerificationDiagnostics {
            adapter: ADAPTER.to_string(),
            complete: true,
            entries: Vec::new(),
            limitation: None,
        });
    }
    Some(parse_pytest_output(
        stdout,
        stderr,
        &check.working_directory,
        false,
    ))
}

fn supports_pytest(check: &ValidatedCheck) -> bool {
    if check.id() != "python.tests" || check.role != super::VerificationRole::Test {
        return false;
    }
    let program_is_python = match &check.program {
        ValidatedProgram::Bare(program) => {
            program == "python" || program == "python3" || program.ends_with("/python")
        }
        ValidatedProgram::RepositoryRelative(program) => program
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "python" || name == "python3"),
    };
    let args_have_pytest = check
        .args
        .iter()
        .any(|arg| arg == "pytest" || arg.ends_with("/pytest"));
    program_is_python && args_have_pytest
}

pub(crate) fn parse_pytest_output(
    stdout: &str,
    stderr: &str,
    working_directory: &Path,
    truncated: bool,
) -> VerificationDiagnostics {
    if truncated {
        return unavailable("pytest output was truncated");
    }
    let cwd = working_directory;
    let mut entries = Vec::new();
    for line in stdout.lines().chain(stderr.lines()) {
        let trimmed = line.trim_start();
        if let Some(path) = trimmed
            .strip_prefix("ImportError while loading conftest '")
            .and_then(|value| value.strip_suffix("'."))
            .or_else(|| {
                trimmed
                    .strip_prefix("ImportError while loading conftest \"")
                    .and_then(|value| value.strip_suffix("\"."))
            })
            && let Some(path) = normalize_path(path, cwd)
        {
            entries.push(VerificationDiagnostic {
                kind: VerificationDiagnosticKind::CollectionError,
                key: format!("python.tests:{path}:collection-error"),
            });
        }

        let Some(rest) = trimmed
            .strip_prefix("FAILED ")
            .or_else(|| trimmed.strip_prefix("ERROR "))
        else {
            continue;
        };
        let rest = rest.strip_prefix("collecting ").unwrap_or(rest);
        let token = rest.split_once(" - ").map_or(rest, |(node, _)| node).trim();
        if token.is_empty() {
            continue;
        }
        let (node, kind, suffix) = if let Some((path, test)) = token.split_once("::") {
            let Some(path) = normalize_path(path, cwd) else {
                continue;
            };
            (
                format!("{path}::{test}"),
                VerificationDiagnosticKind::FailedTestNode,
                "failed",
            )
        } else {
            let Some(path) = normalize_path(token, cwd) else {
                continue;
            };
            (
                path,
                VerificationDiagnosticKind::CollectionError,
                "collection-error",
            )
        };
        entries.push(VerificationDiagnostic {
            kind,
            key: format!("python.tests:{node}:{suffix}"),
        });
    }

    entries.sort_by(|left, right| left.key.cmp(&right.key));
    entries.dedup_by(|left, right| left.key == right.key);
    if entries.is_empty() {
        unavailable("pytest output did not contain a supported diagnostic")
    } else {
        VerificationDiagnostics {
            adapter: ADAPTER.to_string(),
            complete: true,
            entries,
            limitation: None,
        }
    }
}

fn unavailable(reason: &str) -> VerificationDiagnostics {
    VerificationDiagnostics {
        adapter: ADAPTER.to_string(),
        complete: false,
        entries: Vec::new(),
        limitation: Some(reason.to_string()),
    }
}

fn normalize_path(raw: &str, cwd: &Path) -> Option<String> {
    let raw = raw.replace('\\', "/");
    let candidate = PathBuf::from(&raw);
    let relative = if candidate.is_absolute() {
        candidate.strip_prefix(cwd).ok()?.to_path_buf()
    } else {
        candidate
    };
    let normalized = relative.to_string_lossy().replace('\\', "/");
    let normalized = normalized.strip_prefix("./").unwrap_or(&normalized);
    if normalized.is_empty()
        || normalized == "."
        || normalized.starts_with("../")
        || normalized.contains("/../")
    {
        None
    } else {
        Some(normalized.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pytest_failure_emits_normalized_exact_node() {
        let diagnostics = parse_pytest_output(
            "",
            "FAILED tests/test_api.py::test_create[param] - AssertionError\n",
            Path::new("/workspace"),
            false,
        );

        assert_eq!(diagnostics.adapter, "pytest-node-v1");
        assert!(diagnostics.complete);
        assert_eq!(diagnostics.entries.len(), 1);
        assert_eq!(
            diagnostics.entries[0].kind,
            VerificationDiagnosticKind::FailedTestNode
        );
        assert_eq!(
            diagnostics.entries[0].key,
            "python.tests:tests/test_api.py::test_create[param]:failed"
        );
    }

    #[test]
    fn pytest_collection_error_is_a_distinct_diagnostic() {
        let diagnostics = parse_pytest_output(
            "ImportError while loading conftest '/workspace/tests/conftest.py'.\n",
            "",
            Path::new("/workspace"),
            false,
        );

        assert!(diagnostics.complete);
        assert_eq!(
            diagnostics.entries[0].kind,
            VerificationDiagnosticKind::CollectionError
        );
        assert_eq!(
            diagnostics.entries[0].key,
            "python.tests:tests/conftest.py:collection-error"
        );
    }

    #[test]
    fn unsupported_or_truncated_output_stays_unavailable() {
        let unsupported = parse_pytest_output("pytest crashed", "", Path::new("/workspace"), false);
        assert!(!unsupported.complete);
        assert!(unsupported.entries.is_empty());
        assert_eq!(
            unsupported.limitation.as_deref(),
            Some("pytest output did not contain a supported diagnostic")
        );

        let truncated = parse_pytest_output(
            "",
            "FAILED tests/test_api.py::test_create - AssertionError\n",
            Path::new("/workspace"),
            true,
        );
        assert!(!truncated.complete);
        assert_eq!(
            truncated.limitation.as_deref(),
            Some("pytest output was truncated")
        );
    }

    #[test]
    fn paths_outside_working_directory_do_not_become_exact_keys() {
        let diagnostics = parse_pytest_output(
            "",
            "FAILED /other/tests/test_api.py::test_create - AssertionError\n",
            Path::new("/workspace"),
            false,
        );

        assert!(!diagnostics.complete);
        assert!(diagnostics.entries.is_empty());
    }

    #[test]
    fn pytest_node_parameters_with_spaces_remain_one_identity() {
        let diagnostics = parse_pytest_output(
            "",
            "FAILED tests/test_api.py::test_create[a b] - AssertionError\n",
            Path::new("/workspace"),
            false,
        );

        assert!(diagnostics.complete);
        assert_eq!(
            diagnostics.entries[0].key,
            "python.tests:tests/test_api.py::test_create[a b]:failed"
        );
    }
}
