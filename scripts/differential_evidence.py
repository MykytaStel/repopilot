"""Normalize allowlisted baseline diagnostics into deterministic evidence IDs."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any

from differential_identity import REVIEW_COMPARISON_SCHEME


_COMPILE_FILE = re.compile(r"\*\*\* Error compiling ['\"](?P<path>.+?)['\"]")
_PYTHON_FILE = re.compile(r'^\s*File ["\'](?P<path>.+?)["\'], line (?P<line>\d+)')
_PYTHON_ERROR = re.compile(r"^\s*(?P<kind>[A-Za-z_]\w*(?:Error|Warning)):")
_PYTEST_FAILURE = re.compile(
    r"^(?P<status>FAILED|ERROR)\s+(?:(?:collecting)\s+)?(?P<node>\S+)", re.MULTILINE
)
_PYTEST_CONFTEST_ERROR = re.compile(
    r"ImportError while loading conftest ['\"](?P<path>.+?)['\"]\."
)
_COMPARISON_SCHEME = REVIEW_COMPARISON_SCHEME
_REVIEW_VERIFICATION_SCHEME = "review-verification-v1"
_COMPARABLE_COMPILE_KINDS = {"SyntaxError"}
_PYTEST_COMPARISON_UNAVAILABLE = (
    "python.tests review has no exact test-node failure identity; node paths alone are not comparable"
)
_PYTEST_VERIFICATION_UNAVAILABLE = (
    "python.tests requires an explicit python.tests verification with revision-compatible, complete output"
)


def _text(value: bytes | str) -> str:
    return value.decode("utf-8", errors="replace") if isinstance(value, bytes) else value


def _relative_path(raw_path: str, cwd: Path) -> str:
    candidate = Path(raw_path.replace("\\", "/"))
    try:
        if candidate.is_absolute():
            # Keep lexical paths stable even when the host exposes `/tmp` via
            # a symlink (as macOS does); resolving one side would lose that
            # relationship and collapse the evidence to a basename.
            candidate = candidate.relative_to(Path(cwd))
    except ValueError:
        candidate = Path(candidate.name)
    normalized = candidate.as_posix()
    if normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized or "."


def _comparison_path_is_confined(raw_path: str, cwd: Path) -> bool:
    candidate = Path(raw_path.replace("\\", "/"))
    if not candidate.is_absolute():
        return True
    try:
        candidate.relative_to(Path(cwd))
    except ValueError:
        return False
    return True


def _unavailable(reason: str) -> dict[str, Any]:
    return {"status": "unavailable", "reason": reason}


def _measured(
    source: str,
    keys: list[str],
    comparison_keys: list[str] | None = None,
    comparison_reason: str | None = None,
) -> dict[str, Any]:
    comparison: dict[str, Any]
    if comparison_reason is not None:
        comparison = {
            "status": "unavailable",
            "reason": comparison_reason,
            "scheme": _COMPARISON_SCHEME,
        }
    elif comparison_keys is None and not keys:
        comparison = {"status": "measured", "keys": [], "scheme": _COMPARISON_SCHEME}
    elif comparison_keys is None:
        comparison = {
            "status": "unavailable",
            "reason": "baseline evidence has no review-comparable identity mapping",
            "scheme": _COMPARISON_SCHEME,
        }
    else:
        comparison = {
            "status": "measured",
            "keys": comparison_keys,
            "scheme": _COMPARISON_SCHEME,
        }
    return {"status": "measured", "keys": keys, "source": source, "comparison": comparison}


def _compile_evidence(stdout: str, stderr: str, returncode: int, cwd: Path) -> dict[str, Any]:
    if returncode == 0:
        return _measured("python.compile-v1", [])
    text = "\n".join((stdout, stderr))
    current_path: str | None = None
    current_line: str | None = None
    current_path_comparable = False
    keys: set[str] = set()
    comparison_keys: set[str] = set()
    for line in text.splitlines():
        file_match = _COMPILE_FILE.search(line)
        if file_match:
            raw_path = file_match.group("path")
            current_path = _relative_path(raw_path, cwd)
            current_path_comparable = _comparison_path_is_confined(raw_path, cwd)
            current_line = None
            continue
        location_match = _PYTHON_FILE.match(line)
        if location_match:
            raw_path = location_match.group("path")
            current_path = _relative_path(raw_path, cwd)
            current_path_comparable = _comparison_path_is_confined(raw_path, cwd)
            current_line = location_match.group("line")
            continue
        error_match = _PYTHON_ERROR.match(line)
        if error_match and current_path and current_line:
            kind = error_match.group("kind")
            key = f"python.compile:{current_path}:{current_line}:{kind}"
            keys.add(key)
            if current_path_comparable:
                comparison_keys.add(key)
    if not keys:
        return _unavailable("python.compile output did not contain a supported diagnostic")
    normalized_keys = sorted(keys)
    if comparison_keys == keys and all(
        kind in _COMPARABLE_COMPILE_KINDS for kind in _compile_kinds(normalized_keys)
    ):
        return _measured("python.compile-v1", normalized_keys, normalized_keys)
    return _measured("python.compile-v1", normalized_keys)


def _compile_kinds(keys: list[str]) -> set[str]:
    return {key.rsplit(":", 1)[-1] for key in keys}


def _pytest_evidence(stdout: str, stderr: str, returncode: int, cwd: Path) -> dict[str, Any]:
    if returncode == 0:
        return _measured("python.tests-v1", [])
    text = "\n".join((stdout, stderr))
    keys = _pytest_failure_keys(text, cwd)
    if not keys:
        return _unavailable("python.tests output did not contain a supported diagnostic")
    return _measured(
        "python.tests-v1",
        sorted(keys),
        comparison_reason=_PYTEST_COMPARISON_UNAVAILABLE,
    )


def _pytest_failure_keys(text: str, cwd: Path) -> set[str]:
    keys: set[str] = set()
    for match in _PYTEST_CONFTEST_ERROR.finditer(text):
        path = _relative_path(match.group("path"), cwd)
        keys.add(f"python.tests:{path}:collection-error")
    for match in _PYTEST_FAILURE.finditer(text):
        node = match.group("node")
        if "::" in node:
            path, test_name = node.split("::", 1)
            normalized_node = f"{_relative_path(path, cwd)}::{test_name}"
            status = "failed"
        else:
            normalized_node = _relative_path(node, cwd)
            status = "collection-error"
        keys.add(f"python.tests:{normalized_node}:{status}")
    return keys


def normalize_review_verification(report: dict[str, Any], cwd: Path) -> dict[str, Any]:
    """Normalize an explicit, revision-compatible pytest verification result.

    This is deliberately separate from static review identities. An exact node
    overlap is comparable only when RepoPilot actually ran the configured
    `python.tests` check and retained complete output for the same revision.
    """

    outcomes = report.get("verification")
    if outcomes is None:
        readiness = report.get("merge_readiness")
        outcomes = readiness.get("verification") if isinstance(readiness, dict) else None
    if not isinstance(outcomes, list):
        return _unavailable(_PYTEST_VERIFICATION_UNAVAILABLE)
    selected = [
        outcome
        for outcome in outcomes
        if isinstance(outcome, dict) and outcome.get("check_id") == "python.tests"
    ]
    if len(selected) != 1:
        return _unavailable(_PYTEST_VERIFICATION_UNAVAILABLE)
    outcome = selected[0]
    if outcome.get("role") != "test" or outcome.get("revision_compatible") is not True:
        return _unavailable(_PYTEST_VERIFICATION_UNAVAILABLE)
    if outcome.get("stdout_truncated") is not False or outcome.get("stderr_truncated") is not False:
        return _unavailable(_PYTEST_VERIFICATION_UNAVAILABLE)
    status = outcome.get("status")
    if status == "passed":
        return {
            "status": "measured",
            "keys": [],
            "scheme": _REVIEW_VERIFICATION_SCHEME,
        }
    if status != "failed":
        return _unavailable(_PYTEST_VERIFICATION_UNAVAILABLE)
    stdout = outcome.get("stdout_excerpt")
    stderr = outcome.get("stderr_excerpt")
    if not isinstance(stdout, str) or not isinstance(stderr, str):
        return _unavailable(_PYTEST_VERIFICATION_UNAVAILABLE)
    keys = _pytest_failure_keys("\n".join((stdout, stderr)), cwd)
    if not keys:
        return _unavailable("python.tests verification output did not contain a supported diagnostic")
    return {
        "status": "measured",
        "keys": sorted(keys),
        "scheme": _REVIEW_VERIFICATION_SCHEME,
    }


def normalize_baseline_evidence(
    baseline_id: str,
    stdout: bytes | str,
    stderr: bytes | str,
    returncode: int | None,
    cwd: Path,
) -> dict[str, Any]:
    """Return measured normalized IDs or an explicit unavailable status.

    A successful check with no diagnostics is still measured with an empty set.
    Non-zero output is measured only when the adapter can identify a stable
    path/node diagnostic; raw command output is never persisted in the artifact.
    """

    if baseline_id not in {"python.compile", "python.tests"}:
        return _unavailable(f"no adapter registered for baseline {baseline_id}")
    if returncode is None:
        return _unavailable("baseline command did not complete")
    text_stdout, text_stderr = _text(stdout), _text(stderr)
    if baseline_id == "python.compile":
        return _compile_evidence(text_stdout, text_stderr, returncode, cwd)
    return _pytest_evidence(text_stdout, text_stderr, returncode, cwd)
