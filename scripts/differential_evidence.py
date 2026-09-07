"""Normalize allowlisted baseline diagnostics into deterministic evidence IDs."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any


_COMPILE_FILE = re.compile(r"\*\*\* Error compiling ['\"](?P<path>.+?)['\"]")
_PYTHON_FILE = re.compile(r'^\s*File ["\'](?P<path>.+?)["\'], line (?P<line>\d+)')
_PYTHON_ERROR = re.compile(r"^\s*(?P<kind>[A-Za-z_]\w*(?:Error|Warning)):")
_PYTEST_FAILURE = re.compile(
    r"^(?P<status>FAILED|ERROR)\s+(?:(?:collecting)\s+)?(?P<node>\S+)", re.MULTILINE
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


def _unavailable(reason: str) -> dict[str, Any]:
    return {"status": "unavailable", "reason": reason}


def _compile_evidence(stdout: str, stderr: str, returncode: int, cwd: Path) -> dict[str, Any]:
    if returncode == 0:
        return {"status": "measured", "keys": [], "source": "python.compile-v1"}
    text = "\n".join((stdout, stderr))
    current_path: str | None = None
    current_line: str | None = None
    keys: set[str] = set()
    for line in text.splitlines():
        file_match = _COMPILE_FILE.search(line)
        if file_match:
            current_path = _relative_path(file_match.group("path"), cwd)
            current_line = None
            continue
        location_match = _PYTHON_FILE.match(line)
        if location_match:
            current_path = _relative_path(location_match.group("path"), cwd)
            current_line = location_match.group("line")
            continue
        error_match = _PYTHON_ERROR.match(line)
        if error_match and current_path and current_line:
            keys.add(f"python.compile:{current_path}:{current_line}:{error_match.group('kind')}")
    if not keys:
        return _unavailable("python.compile output did not contain a supported diagnostic")
    return {"status": "measured", "keys": sorted(keys), "source": "python.compile-v1"}


def _pytest_evidence(stdout: str, stderr: str, returncode: int, cwd: Path) -> dict[str, Any]:
    if returncode == 0:
        return {"status": "measured", "keys": [], "source": "python.tests-v1"}
    text = "\n".join((stdout, stderr))
    keys: set[str] = set()
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
    if not keys:
        return _unavailable("python.tests output did not contain a supported diagnostic")
    return {"status": "measured", "keys": sorted(keys), "source": "python.tests-v1"}


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
