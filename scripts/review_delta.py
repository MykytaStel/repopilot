#!/usr/bin/env python3
"""Build a deterministic finding delta from base and head scan reports."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


PRIORITY_ORDER = {"P0": 0, "P1": 1, "P2": 2, "P3": 3}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--head", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--base-revision", default="")
    parser.add_argument("--head-revision", default="")
    return parser.parse_args()


def load_findings(path: Path) -> list[dict[str, Any]]:
    report = json.loads(path.read_text(encoding="utf-8"))
    findings = report.get("findings")
    if not isinstance(findings, list):
        raise ValueError(f"{path} does not contain a findings array")
    if not all(isinstance(finding, dict) for finding in findings):
        raise ValueError(f"{path} contains a non-object finding")
    return findings


def finding_id(finding: dict[str, Any]) -> str:
    value = finding.get("id")
    return value if isinstance(value, str) else ""


def occurrence_identity(finding: dict[str, Any]) -> str:
    occurrence_key = finding.get("occurrence_key")
    if isinstance(occurrence_key, str) and occurrence_key:
        return occurrence_key
    evidence = finding.get("evidence", [])
    rendered_evidence = json.dumps(
        evidence, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    )
    return f"{finding_id(finding)}:{rendered_evidence}"


def location(finding: dict[str, Any]) -> tuple[str, int]:
    evidence = finding.get("evidence")
    if not isinstance(evidence, list) or not evidence or not isinstance(evidence[0], dict):
        return "", 0
    path = evidence[0].get("path")
    line = evidence[0].get("line_start")
    return (
        path if isinstance(path, str) else "",
        line if isinstance(line, int) else 0,
    )


def finding_sort_key(finding: dict[str, Any]) -> tuple[int, str, int, str, str]:
    risk = finding.get("risk")
    priority = risk.get("priority") if isinstance(risk, dict) else None
    path, line = location(finding)
    return (
        PRIORITY_ORDER.get(priority, len(PRIORITY_ORDER)),
        path,
        line,
        finding_id(finding),
        occurrence_identity(finding),
    )


def build_delta(
    base_findings: list[dict[str, Any]],
    head_findings: list[dict[str, Any]],
    base_revision: str = "",
    head_revision: str = "",
) -> dict[str, Any]:
    base_ids = {finding_id(finding) for finding in base_findings}
    head_ids = {finding_id(finding) for finding in head_findings}
    base_occurrences = {occurrence_identity(finding) for finding in base_findings}
    head_occurrences = {occurrence_identity(finding) for finding in head_findings}

    new_findings = []
    changed_findings = []
    unchanged_count = 0
    for finding in head_findings:
        if occurrence_identity(finding) in base_occurrences:
            unchanged_count += 1
        elif finding_id(finding) in base_ids:
            changed_findings.append(finding)
        else:
            new_findings.append(finding)

    resolved_findings = [
        finding
        for finding in base_findings
        if occurrence_identity(finding) not in head_occurrences
        and finding_id(finding) not in head_ids
    ]

    new_findings.sort(key=finding_sort_key)
    changed_findings.sort(key=finding_sort_key)
    resolved_findings.sort(key=finding_sort_key)
    return {
        "schema_version": "1",
        "report": {"kind": "review-delta", "schema_version": "1"},
        "base_revision": base_revision,
        "head_revision": head_revision,
        "summary": {
            "new_findings": len(new_findings),
            "changed_findings": len(changed_findings),
            "resolved_findings": len(resolved_findings),
            "unchanged_findings": unchanged_count,
        },
        "new_findings": new_findings,
        "changed_findings": changed_findings,
        "resolved_findings": resolved_findings,
    }


def main() -> None:
    args = parse_args()
    delta = build_delta(
        load_findings(args.base),
        load_findings(args.head),
        args.base_revision,
        args.head_revision,
    )
    args.output.write_text(
        json.dumps(delta, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
