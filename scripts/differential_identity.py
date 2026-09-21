"""Review-side identities that have an explicit baseline mapping contract."""

from __future__ import annotations

from pathlib import Path, PurePosixPath
from typing import Any


REVIEW_COMPARISON_SCHEME = "review-exact-v1"


def _relative_report_path(raw_path: object, root_path: object) -> str | None:
    if not isinstance(raw_path, str) or not raw_path:
        return None
    path = raw_path.replace("\\", "/")
    root = root_path.replace("\\", "/") if isinstance(root_path, str) else ""
    candidate = Path(path)
    if candidate.is_absolute():
        if not root:
            return None
        try:
            candidate = candidate.relative_to(Path(root))
        except ValueError:
            return None
    normalized = PurePosixPath(candidate.as_posix())
    if normalized == PurePosixPath(".") or ".." in normalized.parts:
        return None
    return normalized.as_posix()


def _changed_ranges(report: dict[str, Any]) -> dict[str, list[tuple[int, int]]]:
    changed_files = report.get("changed_files")
    if not isinstance(changed_files, list):
        return {}
    ranges_by_path: dict[str, list[tuple[int, int]]] = {}
    for changed in changed_files:
        if not isinstance(changed, dict):
            continue
        path = changed.get("path")
        if not isinstance(path, str) or not path:
            continue
        ranges = changed.get("ranges")
        if not isinstance(ranges, list):
            continue
        normalized_ranges = []
        for changed_range in ranges:
            if not isinstance(changed_range, dict):
                continue
            start = changed_range.get("start")
            end = changed_range.get("end")
            if isinstance(start, int) and isinstance(end, int) and 0 < start <= end:
                normalized_ranges.append((start, end))
        if normalized_ranges:
            ranges_by_path[path.replace("\\", "/")] = normalized_ranges
    return ranges_by_path


def review_comparable_diagnostic_keys(report: dict[str, Any]) -> list[str]:
    """Return exact baseline identities for explicitly supported review diagnostics.

    The mapping is intentionally narrow: only RepoPilot's Python syntax diagnostic,
    on a line added by the reviewed diff, maps to the Python compile adapter's
    identity. Unknown codes, missing locations, and paths outside the review root
    remain incomparable instead of becoming path-level guesses.
    """

    ranges_by_path = _changed_ranges(report)
    diagnostics = report.get("diagnostics")
    if not isinstance(diagnostics, list):
        return []
    keys: set[str] = set()
    root_path = report.get("root_path")
    for diagnostic in diagnostics:
        if not isinstance(diagnostic, dict) or diagnostic.get("code") != "python.syntax-error":
            continue
        path = _relative_report_path(diagnostic.get("path"), root_path)
        line = diagnostic.get("line")
        if path is None or not isinstance(line, int) or line < 1:
            continue
        ranges = ranges_by_path.get(path)
        if ranges is None or not any(start <= line <= end for start, end in ranges):
            continue
        if not path.endswith(".py"):
            continue
        keys.add(f"python.compile:{path}:{line}:SyntaxError")
    return sorted(keys)
