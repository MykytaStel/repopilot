#!/usr/bin/env python3
"""Validate RepoPilot's held-out recall corpus contract.

The corpus is intentionally separate from rule fixtures and zoo labels.  This
module validates the evidence manifest; executing the scanner is a later step.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path


MANIFEST_SCHEMA_VERSION = 1
ALLOWED_PROFILES = {"default", "strict"}
ALLOWED_OUTCOMES = {"must-fire", "must-not-fire"}
ALLOWED_KINDS = {"seeded-defect", "safe-guard"}
CASE_ID = re.compile(r"^[a-z0-9][a-z0-9._-]+$")
TOP_LEVEL_FIELDS = {"schema_version", "corpus", "case"}
CASE_FIELDS = {"id", "rule_id", "path", "profile", "outcome", "kind", "language", "rationale"}


class RecallManifestError(ValueError):
    """Raised when a recall corpus violates its evidence contract."""


@dataclass(frozen=True)
class RecallCase:
    case_id: str
    rule_id: str
    path: str
    profile: str
    outcome: str
    kind: str
    language: str
    rationale: str


def _required_string(raw: dict[str, object], field: str, index: int) -> str:
    value = raw.get(field)
    if not isinstance(value, str) or not value.strip():
        raise RecallManifestError(f"case {index}: {field} must be a non-empty string")
    return value.strip()


def load_manifest(path: Path) -> tuple[str, list[RecallCase]]:
    try:
        document = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise RecallManifestError(f"cannot read manifest {path}: {error}") from error

    unknown_fields = set(document) - TOP_LEVEL_FIELDS
    if unknown_fields:
        raise RecallManifestError(f"unknown top-level fields: {', '.join(sorted(unknown_fields))}")
    if document.get("schema_version") != MANIFEST_SCHEMA_VERSION:
        raise RecallManifestError(
            f"schema_version must be {MANIFEST_SCHEMA_VERSION}"
        )
    corpus = document.get("corpus")
    if not isinstance(corpus, str) or not corpus.strip():
        raise RecallManifestError("corpus must be a non-empty string")
    raw_cases = document.get("case")
    if not isinstance(raw_cases, list) or not raw_cases:
        raise RecallManifestError("manifest must contain at least one [[case]]")

    cases: list[RecallCase] = []
    for index, raw in enumerate(raw_cases, start=1):
        if not isinstance(raw, dict):
            raise RecallManifestError(f"case {index}: entry must be a table")
        unknown_fields = set(raw) - CASE_FIELDS
        if unknown_fields:
            raise RecallManifestError(
                f"case {index}: unknown fields: {', '.join(sorted(unknown_fields))}"
            )
        cases.append(
            RecallCase(
                case_id=_required_string(raw, "id", index),
                rule_id=_required_string(raw, "rule_id", index),
                path=_required_string(raw, "path", index),
                profile=_required_string(raw, "profile", index),
                outcome=_required_string(raw, "outcome", index),
                kind=_required_string(raw, "kind", index),
                language=_required_string(raw, "language", index),
                rationale=_required_string(raw, "rationale", index),
            )
        )
    return corpus.strip(), cases


def validate_manifest(
    manifest_path: Path,
    rules_reference_path: Path,
    fixture_root: Path,
) -> tuple[str, list[RecallCase]]:
    corpus, cases = load_manifest(manifest_path)
    try:
        rule_text = rules_reference_path.read_text(encoding="utf-8")
    except OSError as error:
        raise RecallManifestError(
            f"cannot read rule reference {rules_reference_path}: {error}"
        ) from error
    known_rules = set(re.findall(r"^### `([^`]+)`", rule_text, re.MULTILINE))
    if not known_rules:
        raise RecallManifestError("rule reference contains no rule IDs")

    manifest_root = manifest_path.parent.resolve()
    fixture_root = fixture_root.resolve()
    ids: set[str] = set()
    locations: set[tuple[str, str]] = set()
    seeded_rules: set[str] = set()
    for index, case in enumerate(cases, start=1):
        if not CASE_ID.fullmatch(case.case_id):
            raise RecallManifestError(f"case {index}: invalid id {case.case_id!r}")
        if case.case_id in ids:
            raise RecallManifestError(f"duplicate case id: {case.case_id}")
        ids.add(case.case_id)
        if case.rule_id not in known_rules:
            raise RecallManifestError(f"case {case.case_id}: unknown rule_id {case.rule_id}")
        if case.profile not in ALLOWED_PROFILES:
            raise RecallManifestError(f"case {case.case_id}: invalid profile {case.profile}")
        if case.outcome not in ALLOWED_OUTCOMES:
            raise RecallManifestError(f"case {case.case_id}: invalid outcome {case.outcome}")
        if case.kind not in ALLOWED_KINDS:
            raise RecallManifestError(f"case {case.case_id}: invalid kind {case.kind}")
        expected_outcome = "must-fire" if case.kind == "seeded-defect" else "must-not-fire"
        if case.outcome != expected_outcome:
            raise RecallManifestError(
                f"case {case.case_id}: {case.kind} requires outcome {expected_outcome}"
            )
        if Path(case.path).is_absolute():
            raise RecallManifestError(f"case {case.case_id}: path must be relative")
        resolved = (manifest_root / case.path).resolve()
        try:
            resolved.relative_to(fixture_root)
        except ValueError as error:
            raise RecallManifestError(
                f"case {case.case_id}: path escapes recall fixture root"
            ) from error
        if not resolved.is_dir():
            raise RecallManifestError(f"case {case.case_id}: fixture directory missing: {case.path}")
        if not any(candidate.is_file() for candidate in resolved.rglob("*")):
            raise RecallManifestError(f"case {case.case_id}: fixture directory is empty: {case.path}")
        location = (str(resolved), case.profile)
        if location in locations:
            raise RecallManifestError(f"duplicate case location: {case.path} ({case.profile})")
        locations.add(location)
        if case.kind == "seeded-defect":
            seeded_rules.add(case.rule_id)

    if not seeded_rules:
        raise RecallManifestError("manifest must contain at least one seeded-defect case")
    if not any(case.kind == "safe-guard" for case in cases):
        raise RecallManifestError("manifest must contain at least one safe-guard case")
    return corpus, cases


def summary(corpus: str, cases: list[RecallCase]) -> dict[str, object]:
    return {
        "corpus": corpus,
        "schema_version": MANIFEST_SCHEMA_VERSION,
        "cases": len(cases),
        "seeded_defects": sum(case.kind == "seeded-defect" for case in cases),
        "safe_guards": sum(case.kind == "safe-guard" for case in cases),
        "rules": sorted({case.rule_id for case in cases}),
        "languages": sorted({case.language for case in cases}),
        "profiles": sorted({case.profile for case in cases}),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=Path("tests/recall/manifest.toml"))
    parser.add_argument("--rules-reference", type=Path, default=Path("docs/rules-reference.md"))
    parser.add_argument("--fixture-root", type=Path, default=Path("tests/fixtures/recall"))
    parser.add_argument("--format", choices=("text", "json"), default="text")
    args = parser.parse_args(argv)
    try:
        corpus, cases = validate_manifest(args.manifest, args.rules_reference, args.fixture_root)
    except RecallManifestError as error:
        print(f"recall manifest invalid: {error}", file=sys.stderr)
        return 1
    data = summary(corpus, cases)
    if args.format == "json":
        print(json.dumps(data, indent=2, sort_keys=True))
    else:
        print(f"Recall corpus: {data['corpus']}")
        print(f"Cases: {data['cases']} ({data['seeded_defects']} seeded defects, {data['safe_guards']} safe guards)")
        print(f"Rules: {', '.join(data['rules'])}")
        print(f"Languages: {', '.join(data['languages'])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
