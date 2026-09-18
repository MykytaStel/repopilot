#!/usr/bin/env python3
"""Evaluate an explicitly selected, stable-rule CI policy against scan JSON."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
import tomllib
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "repopilot-scoped-policy-1"
PROFILES = {"default", "strict"}
PRIORITIES = ("p0", "p1", "p2", "p3")
RULE_ID = re.compile(r"^[a-z0-9][a-z0-9._-]*$")
POLICY_KEYS = {
    "schema_version",
    "policy_id",
    "owner",
    "profile",
    "max_priority",
    "rules",
    "rollback",
}


class PolicyError(ValueError):
    """Raised when the explicitly supplied policy is invalid."""


class ReportUnavailable(ValueError):
    """Raised when a report cannot support a safe policy decision."""


def sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        dir=path.parent,
        prefix=f".{path.name}.",
        delete=False,
    ) as handle:
        json.dump(value, handle, indent=2, sort_keys=True)
        handle.write("\n")
        temporary = Path(handle.name)
    os.replace(temporary, path)


def non_empty_string(value: object, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise PolicyError(f"{field} must be a non-empty string")
    return value.strip()


def load_policy(path: Path) -> dict[str, Any]:
    try:
        with path.open("rb") as handle:
            raw = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise PolicyError(f"cannot read policy: {error}") from error
    if not isinstance(raw, dict):
        raise PolicyError("policy must be a TOML table")
    unknown = sorted(set(raw) - POLICY_KEYS)
    if unknown:
        raise PolicyError(f"policy has unknown fields: {', '.join(unknown)}")
    if raw.get("schema_version") != 1:
        raise PolicyError("policy schema_version must be 1")
    policy_id = non_empty_string(raw.get("policy_id"), "policy_id")
    owner = non_empty_string(raw.get("owner"), "owner")
    profile = non_empty_string(raw.get("profile"), "profile")
    if profile not in PROFILES:
        raise PolicyError("policy profile must be default or strict")
    max_priority = non_empty_string(raw.get("max_priority"), "max_priority").lower()
    if max_priority not in PRIORITIES:
        raise PolicyError("policy max_priority must be p0, p1, p2, or p3")
    rules = raw.get("rules")
    if (
        not isinstance(rules, list)
        or not rules
        or not all(isinstance(rule, str) for rule in rules)
    ):
        raise PolicyError("policy rules must be a non-empty string array")
    normalized_rules = sorted({rule.strip() for rule in rules})
    if not all(RULE_ID.fullmatch(rule) for rule in normalized_rules):
        raise PolicyError("policy rules contain an invalid rule id")
    rollback = non_empty_string(raw.get("rollback"), "rollback")
    return {
        "schema_version": 1,
        "policy_id": policy_id,
        "owner": owner,
        "profile": profile,
        "max_priority": max_priority,
        "rules": normalized_rules,
        "rollback": rollback,
    }


def load_report(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ReportUnavailable(f"cannot read report JSON: {error}") from error
    if not isinstance(value, dict):
        raise ReportUnavailable("report must be a JSON object")
    profile = value.get("visibility_profile")
    if profile not in PROFILES:
        raise ReportUnavailable("report visibility_profile is missing or invalid")
    findings = value.get("findings")
    if not isinstance(findings, list):
        raise ReportUnavailable("report findings must be an array")
    return value


def repository_revision(repo: Path) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo), "rev-parse", "HEAD"],
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.TimeoutExpired):
        return "unavailable"
    revision = result.stdout.strip()
    return revision if result.returncode == 0 and revision else "unavailable"


def priority_rank(value: object) -> int:
    if not isinstance(value, str) or value.lower() not in PRIORITIES:
        raise ReportUnavailable("scoped finding has missing or invalid risk priority")
    return PRIORITIES.index(value.lower())


def finding_location(finding: dict[str, Any]) -> tuple[str | None, int | None]:
    evidence = finding.get("evidence")
    if (
        not isinstance(evidence, list)
        or not evidence
        or not isinstance(evidence[0], dict)
    ):
        return None, None
    first = evidence[0]
    path = first.get("path") if isinstance(first.get("path"), str) else None
    line = first.get("line_start") if isinstance(first.get("line_start"), int) else None
    return path, line


def evaluate(
    policy: dict[str, Any], report: dict[str, Any]
) -> tuple[str, str, list[dict[str, Any]]]:
    if report["visibility_profile"] != policy["profile"]:
        raise ReportUnavailable(
            f"report profile {report['visibility_profile']!r} does not match policy profile {policy['profile']!r}"
        )
    matches: list[dict[str, Any]] = []
    allowed_rules = set(policy["rules"])
    max_priority = PRIORITIES.index(policy["max_priority"])
    for finding in report["findings"]:
        if not isinstance(finding, dict):
            raise ReportUnavailable("report contains a non-object finding")
        rule_id = finding.get("rule_id")
        if rule_id not in allowed_rules:
            continue
        provenance = finding.get("provenance")
        lifecycle = (
            provenance.get("rule_lifecycle") if isinstance(provenance, dict) else None
        )
        if lifecycle != "stable":
            raise ReportUnavailable(f"scoped rule {rule_id!r} is not marked stable")
        risk = finding.get("risk")
        priority = priority_rank(
            risk.get("priority") if isinstance(risk, dict) else None
        )
        if priority > max_priority:
            continue
        path, line = finding_location(finding)
        matches.append(
            {
                "finding_id": finding.get("id", "unavailable"),
                "rule_id": rule_id,
                "priority": PRIORITIES[priority].upper(),
                "path": path,
                "line": line,
            }
        )
    status = "failed" if matches else "passed"
    reason = (
        f"{len(matches)} scoped finding(s) meet the policy"
        if matches
        else "no scoped findings meet the policy"
    )
    return status, reason, matches


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Evaluate an opt-in stable-rule RepoPilot policy."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    evaluate_parser = subparsers.add_parser(
        "evaluate", help="evaluate a scan JSON report"
    )
    evaluate_parser.add_argument("--report", required=True, type=Path)
    evaluate_parser.add_argument("--policy", required=True, type=Path)
    evaluate_parser.add_argument("--output", required=True, type=Path)
    evaluate_parser.add_argument("--repo", type=Path, default=Path("."))
    evaluate_parser.add_argument(
        "--mode", choices=("advisory", "blocking"), default="advisory"
    )
    return parser.parse_args()


def run(args: argparse.Namespace) -> int:
    policy_hash = sha256_file(args.policy)
    report_hash = sha256_file(args.report)
    base: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "status": "unavailable",
        "mode": args.mode,
        "policy_id": "unavailable",
        "owner": "unavailable",
        "profile": "unavailable",
        "max_priority": "unavailable",
        "rule_allowlist": [],
        "policy_sha256": policy_hash,
        "report_sha256": report_hash,
        "repository_revision": repository_revision(args.repo),
        "matched_count": 0,
        "matched": [],
        "reason": "scoped policy did not run",
        "rollback": "run with --mode advisory or unset the opt-in CI variable",
    }
    try:
        policy = load_policy(args.policy)
        base.update(
            {
                "policy_id": policy["policy_id"],
                "owner": policy["owner"],
                "profile": policy["profile"],
                "max_priority": policy["max_priority"].upper(),
                "rule_allowlist": policy["rules"],
                "rollback": policy["rollback"],
            }
        )
        report = load_report(args.report)
        status, reason, matches = evaluate(policy, report)
        base.update(
            {
                "status": status,
                "reason": reason,
                "matched": matches,
                "matched_count": len(matches),
            }
        )
    except PolicyError as error:
        base.update({"status": "invalid", "reason": str(error)})
    except ReportUnavailable as error:
        base.update({"status": "unavailable", "reason": str(error)})
    write_json(args.output, base)
    print(
        f"RepoPilot scoped policy: {base['status']} ({base['mode']}) — {base['reason']}"
    )
    if args.mode == "advisory":
        return 0
    if base["status"] == "failed":
        return 1
    if base["status"] != "passed":
        return 2
    return 0


if __name__ == "__main__":
    arguments = parse_args()
    raise SystemExit(run(arguments))
