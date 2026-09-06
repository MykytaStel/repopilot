"""Manifest model and validation for the real-history evidence protocol."""

from __future__ import annotations

import re
import tomllib
from dataclasses import dataclass
from pathlib import Path


SCHEMA_VERSION = 1
ALLOWED_LABEL_STATES = {"pending", "adjudicated", "excluded"}
ALLOWED_LABELS = {"defect-present", "no-defect", "uncertain"}
ALLOWED_SOURCE_KINDS = {"merged-pull-request"}
BASELINES = {"python.compile", "python.tests", "python.lint", "python.typecheck"}
CASE_ID = re.compile(r"^[a-z0-9][a-z0-9._-]+$")
REPO = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
SHA = re.compile(r"^[0-9a-f]{40}$")
TOP_LEVEL_FIELDS = {"schema_version", "corpus", "protocol", "case"}
CASE_FIELDS = {
    "id", "repo", "url", "pull_request", "base_sha", "head_sha", "merge_sha",
    "language", "source_kind", "baseline_ids", "label_state", "annotator_a",
    "annotator_b", "adjudicated", "adjudication_rationale", "expected_rule_ids",
    "exclusion_reason",
}


class HoldoutManifestError(ValueError):
    """Raised when the real-history evidence contract is incomplete or unsafe."""


@dataclass(frozen=True)
class HoldoutCase:
    case_id: str
    repo: str
    url: str
    pull_request: int
    base_sha: str
    head_sha: str
    merge_sha: str
    language: str
    source_kind: str
    baseline_ids: tuple[str, ...]
    label_state: str
    annotator_a: str | None
    annotator_b: str | None
    adjudicated: str | None
    adjudication_rationale: str | None
    expected_rule_ids: tuple[str, ...]
    exclusion_reason: str | None


def required_string(raw: dict[str, object], field: str, index: int) -> str:
    value = raw.get(field)
    if not isinstance(value, str) or not value.strip():
        raise HoldoutManifestError(f"case {index}: {field} must be a non-empty string")
    return value.strip()


def optional_string(raw: dict[str, object], field: str) -> str | None:
    value = raw.get(field)
    if value is None:
        return None
    if not isinstance(value, str) or not value.strip():
        raise HoldoutManifestError(f"{field} must be a non-empty string when supplied")
    return value.strip()


def load_manifest(path: Path) -> tuple[str, str, list[HoldoutCase]]:
    try:
        document = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise HoldoutManifestError(f"cannot read manifest {path}: {error}") from error
    if set(document) - TOP_LEVEL_FIELDS:
        unknown = sorted(set(document) - TOP_LEVEL_FIELDS)
        raise HoldoutManifestError(f"unknown top-level fields: {', '.join(unknown)}")
    if document.get("schema_version") != SCHEMA_VERSION:
        raise HoldoutManifestError(f"schema_version must be {SCHEMA_VERSION}")
    corpus = document.get("corpus")
    protocol = document.get("protocol")
    if not isinstance(corpus, str) or not corpus.strip():
        raise HoldoutManifestError("corpus must be a non-empty string")
    if not isinstance(protocol, str) or not protocol.strip():
        raise HoldoutManifestError("protocol must be a non-empty string")
    raw_cases = document.get("case")
    if not isinstance(raw_cases, list) or len(raw_cases) < 2:
        raise HoldoutManifestError("manifest must contain at least two [[case]] entries")
    cases: list[HoldoutCase] = []
    for index, raw in enumerate(raw_cases, start=1):
        if not isinstance(raw, dict):
            raise HoldoutManifestError(f"case {index}: entry must be a table")
        unknown = set(raw) - CASE_FIELDS
        if unknown:
            raise HoldoutManifestError(f"case {index}: unknown fields: {', '.join(sorted(unknown))}")
        baseline_ids = raw.get("baseline_ids")
        if not isinstance(baseline_ids, list) or not baseline_ids or not all(isinstance(item, str) for item in baseline_ids):
            raise HoldoutManifestError(f"case {index}: baseline_ids must be a non-empty string array")
        expected = raw.get("expected_rule_ids", [])
        if not isinstance(expected, list) or not all(isinstance(item, str) for item in expected):
            raise HoldoutManifestError(f"case {index}: expected_rule_ids must be a string array")
        pull_request = raw.get("pull_request")
        if not isinstance(pull_request, int) or pull_request <= 0:
            raise HoldoutManifestError(f"case {index}: pull_request must be a positive integer")
        cases.append(HoldoutCase(
            case_id=required_string(raw, "id", index),
            repo=required_string(raw, "repo", index),
            url=required_string(raw, "url", index),
            pull_request=pull_request,
            base_sha=required_string(raw, "base_sha", index),
            head_sha=required_string(raw, "head_sha", index),
            merge_sha=required_string(raw, "merge_sha", index),
            language=required_string(raw, "language", index),
            source_kind=required_string(raw, "source_kind", index),
            baseline_ids=tuple(baseline_ids),
            label_state=required_string(raw, "label_state", index),
            annotator_a=optional_string(raw, "annotator_a"),
            annotator_b=optional_string(raw, "annotator_b"),
            adjudicated=optional_string(raw, "adjudicated"),
            adjudication_rationale=optional_string(raw, "adjudication_rationale"),
            expected_rule_ids=tuple(expected),
            exclusion_reason=optional_string(raw, "exclusion_reason"),
        ))
    return corpus.strip(), protocol.strip(), cases


def validate_manifest(path: Path, rules_reference: Path, zoo_manifest: Path) -> tuple[str, str, list[HoldoutCase]]:
    corpus, protocol, cases = load_manifest(path)
    known_rules = set(re.findall(r"^### `([^`]+)`", rules_reference.read_text(encoding="utf-8"), re.MULTILINE))
    zoo_repos = {entry["name"] for entry in tomllib.loads(zoo_manifest.read_text(encoding="utf-8")).get("repo", [])}
    if not known_rules:
        raise HoldoutManifestError("rules reference contains no rule IDs")
    ids: set[str] = set()
    repos: set[str] = set()
    for index, case in enumerate(cases, start=1):
        if not CASE_ID.fullmatch(case.case_id) or case.case_id in ids:
            raise HoldoutManifestError(f"case {index}: invalid or duplicate id {case.case_id!r}")
        ids.add(case.case_id)
        if not REPO.fullmatch(case.repo) or case.repo in repos:
            raise HoldoutManifestError(f"case {case.case_id}: invalid or duplicate repo {case.repo}")
        repos.add(case.repo)
        if case.repo in zoo_repos:
            raise HoldoutManifestError(f"case {case.case_id}: repo overlaps precision zoo: {case.repo}")
        if case.url != f"https://github.com/{case.repo}.git":
            raise HoldoutManifestError(f"case {case.case_id}: URL must pin the declared GitHub repo")
        if any(not SHA.fullmatch(value) for value in (case.base_sha, case.head_sha, case.merge_sha)):
            raise HoldoutManifestError(f"case {case.case_id}: revisions must be full lowercase SHA-1 values")
        if case.base_sha == case.head_sha:
            raise HoldoutManifestError(f"case {case.case_id}: base and head revisions must differ")
        if case.source_kind not in ALLOWED_SOURCE_KINDS:
            raise HoldoutManifestError(f"case {case.case_id}: unsupported source_kind {case.source_kind}")
        unknown_baselines = set(case.baseline_ids) - BASELINES
        if unknown_baselines:
            raise HoldoutManifestError(f"case {case.case_id}: unknown baselines: {', '.join(sorted(unknown_baselines))}")
        if case.label_state not in ALLOWED_LABEL_STATES:
            raise HoldoutManifestError(f"case {case.case_id}: invalid label_state {case.label_state}")
        if any(rule_id not in known_rules for rule_id in case.expected_rule_ids):
            raise HoldoutManifestError(f"case {case.case_id}: expected_rule_ids contains an unknown rule")
        labels = (case.annotator_a, case.annotator_b, case.adjudicated)
        if case.label_state == "pending" and any(label is not None for label in labels):
            raise HoldoutManifestError(f"case {case.case_id}: pending case cannot contain labels")
        if case.label_state == "adjudicated":
            if any(label not in ALLOWED_LABELS for label in labels):
                raise HoldoutManifestError(f"case {case.case_id}: adjudicated case needs three valid labels")
            if case.adjudicated == "defect-present" and not case.expected_rule_ids:
                raise HoldoutManifestError(f"case {case.case_id}: defect-present case needs expected_rule_ids")
            if case.annotator_a != case.annotator_b and not case.adjudication_rationale:
                raise HoldoutManifestError(f"case {case.case_id}: disagreement needs adjudication_rationale")
        if case.label_state == "excluded" and not case.exclusion_reason:
            raise HoldoutManifestError(f"case {case.case_id}: excluded case needs exclusion_reason")
    if len(repos) < 2:
        raise HoldoutManifestError("holdout must cover at least two independent repositories")
    return corpus, protocol, cases


def summary(corpus: str, protocol: str, cases: list[HoldoutCase]) -> dict[str, object]:
    return {
        "schema_version": SCHEMA_VERSION,
        "corpus": corpus,
        "protocol": protocol,
        "cases": len(cases),
        "repositories": sorted(case.repo for case in cases),
        "label_states": {state: sum(case.label_state == state for case in cases) for state in sorted(ALLOWED_LABEL_STATES)},
        "baseline_ids": sorted({baseline for case in cases for baseline in case.baseline_ids}),
    }
