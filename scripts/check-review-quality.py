#!/usr/bin/env python3
"""Fail when a review report contains known low-value signal patterns."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


SIDE_EFFECT_KINDS = {
    "behavioral.env-var-introduced",
    "behavioral.fs-write-added",
    "behavioral.network-call-added",
    "behavioral.raw-sql-added",
    "behavioral.subprocess-added",
    "taint.exec",
    "taint.fs-write",
    "taint.network",
    "taint.sql",
}
REMOVED_KINDS = {
    "behavioral.auth-check-removed",
    "behavioral.error-handling-removed",
}
COARSE_CODE_EXTENSIONS = {
    ".bash",
    ".c",
    ".cc",
    ".cpp",
    ".h",
    ".hpp",
    ".php",
    ".rb",
    ".sh",
    ".swift",
    ".zsh",
}
STDLIB_IMPORT_MARKERS = (
    "Imported dependency 'node:",
    "Imported module: import argparse",
    "Imported module: from pathlib",
    "Imported Rust crate/module: use std::",
    "Imported Rust crate/module: use core::",
    "Imported Rust crate/module: use crate::",
    "Imported Rust crate/module: use self::",
    "Imported Rust crate/module: use super::",
)


def load_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise SystemExit(f"review report not found: {path}")
    except json.JSONDecodeError as error:
        raise SystemExit(f"invalid review report JSON: {error}") from error
    if not isinstance(report, dict):
        raise SystemExit("review report must be a JSON object")
    return report


def signals(report: dict[str, Any]) -> list[dict[str, Any]]:
    tiered = report.get("tiered_signals")
    if not isinstance(tiered, dict):
        return []
    result: list[dict[str, Any]] = []
    for tier in ("definitely", "maybe", "noise"):
        values = tiered.get(tier)
        if isinstance(values, list):
            result.extend(value for value in values if isinstance(value, dict))
    return result


def is_test_path(path: str) -> bool:
    normalized = f"/{path.replace(chr(92), '/')}".lower()
    name = Path(path).name.lower()
    return (
        "/tests/" in normalized
        or "/__tests__/" in normalized
        or "/fixtures/" in normalized
        or ".test." in name
        or ".spec." in name
        or name.endswith(("_test.rs", "_tests.rs", "_test.go", "_test.py"))
        or name.startswith("test_")
    )


def quality_failures(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    for signal in signals(report):
        kind = str(signal.get("kind") or "")
        path = str(signal.get("path") or "")
        detail = str(signal.get("detail") or "")
        source = str((signal.get("provenance") or {}).get("signal_source") or "")

        if kind in SIDE_EFFECT_KINDS and is_test_path(path):
            failures.append(f"{kind} must not surface from test/fixture path {path}")
        if (
            kind in REMOVED_KINDS
            and source == "text-heuristic"
            and Path(path).suffix.lower() not in COARSE_CODE_EXTENSIONS
        ):
            failures.append(f"{kind} coarse fallback is not valid for {path}")
        if kind == "behavioral.dependency-import-added" and any(
            marker in detail for marker in STDLIB_IMPORT_MARKERS
        ):
            failures.append(f"standard-library import surfaced at {path}: {detail}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--review-json", required=True, type=Path)
    args = parser.parse_args()

    report = load_report(args.review_json)
    failures = quality_failures(report)
    summary = {
        "signals_total": len(signals(report)),
        "quality_failures": failures,
        "status": "failed" if failures else "passed",
    }
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
