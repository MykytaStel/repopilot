"""Compare review evidence with a base scan for differential novelty."""

from __future__ import annotations

import json
from typing import Any, Iterable


def _finding_key(finding: dict[str, Any]) -> str:
    rule_id = finding.get("rule_id")
    if not isinstance(rule_id, str) or not rule_id:
        return ""
    evidence = finding.get("evidence")
    if not isinstance(evidence, list) or not evidence:
        return rule_id
    locations = []
    for item in evidence:
        if not isinstance(item, dict):
            continue
        locations.append(
            (
                item.get("path", ""),
                item.get("line_start"),
                item.get("line_end"),
                item.get("snippet", ""),
            )
        )
    if not locations:
        return rule_id
    locations.sort(key=lambda item: json.dumps(item, ensure_ascii=False, separators=(",", ":")))
    return json.dumps([rule_id, locations], ensure_ascii=False, separators=(",", ":"))


def evidence_keys(findings: Iterable[dict[str, Any]]) -> set[str]:
    """Return deterministic exact-evidence identities for findings."""

    return {
        key
        for finding in findings
        if isinstance(finding, dict)
        for key in (_finding_key(finding),)
        if key
    }


def novel_evidence_keys(findings: Iterable[dict[str, Any]], base_keys: set[str]) -> list[str]:
    """Return review evidence identities absent from the base scan."""

    return sorted(evidence_keys(findings) - base_keys)


def evidence_rule_ids(keys: Iterable[str]) -> set[str]:
    """Extract rule IDs from exact evidence identities."""

    rule_ids: set[str] = set()
    for key in keys:
        try:
            identity = json.loads(key)
        except json.JSONDecodeError:
            identity = None
        if isinstance(identity, list) and identity and isinstance(identity[0], str):
            rule_ids.add(identity[0])
        elif key:
            rule_ids.add(key)
    return rule_ids
