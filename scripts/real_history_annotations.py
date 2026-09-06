"""Generate and validate independent real-history annotation artifacts."""

from __future__ import annotations

import hashlib
import json
import re
import tomllib
from pathlib import Path
from typing import Any

from real_history_artifact import validate_collection_data
from real_history_contract import (
    ALLOWED_LABELS,
    HoldoutCase,
    HoldoutManifestError,
    validate_manifest,
)


ANNOTATION_SCHEMA_VERSION = 1
ANNOTATION_PROTOCOL = "dual-independent-adjudication-v1"
ANNOTATION_FIELDS = {
    "id",
    "repo",
    "pull_request",
    "base_sha",
    "head_sha",
    "merge_sha",
    "observed_in_diff_rule_ids",
    "baseline_statuses",
    "label",
    "expected_rule_ids",
    "rationale",
}


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_collection(
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, Any]:
    try:
        data = json.loads(collection_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HoldoutManifestError(f"cannot read collection artifact {collection_path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError("collection artifact must be a JSON object")
    validate_collection_data(data, manifest_path, rules_reference, zoo_manifest)
    data["collection_sha256"] = sha256_file(collection_path)
    return data


def _cases_by_id(data: dict[str, Any]) -> dict[str, dict[str, Any]]:
    cases = data.get("cases")
    if not isinstance(cases, list):
        raise HoldoutManifestError("collection cases must be an array")
    return {case["id"]: case for case in cases if isinstance(case, dict) and isinstance(case.get("id"), str)}


def _toml_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def _toml_array(values: list[str]) -> str:
    return "[" + ", ".join(_toml_string(value) for value in values) + "]"


def _worksheet_header(data: dict[str, Any], reviewer: str) -> list[str]:
    return [
        f"schema_version = {ANNOTATION_SCHEMA_VERSION}",
        f"corpus = {_toml_string(data['corpus'])}",
        f"protocol = {_toml_string(data['protocol'])}",
        f"reviewer = {_toml_string(reviewer)}",
        f"manifest_sha256 = {_toml_string(data['manifest_sha256'])}",
        f"collection_sha256 = {_toml_string(data['collection_sha256'])}",
        "",
    ]


def _worksheet_case(case: HoldoutCase, observation: dict[str, Any]) -> list[str]:
    review = observation["review"]
    rule_ids = review.get("in_diff_rule_ids", []) if isinstance(review, dict) else []
    if not isinstance(rule_ids, list) or not all(isinstance(item, str) for item in rule_ids):
        raise HoldoutManifestError(f"collection case {case.case_id}: invalid rule evidence")
    baselines = observation["baselines"]
    statuses = [f"{baseline_id}={baselines[baseline_id]['status']}" for baseline_id in case.baseline_ids]
    return [
        "[[case]]",
        f"id = {_toml_string(case.case_id)}",
        f"repo = {_toml_string(case.repo)}",
        f"pull_request = {case.pull_request}",
        f"base_sha = {_toml_string(case.base_sha)}",
        f"head_sha = {_toml_string(case.head_sha)}",
        f"merge_sha = {_toml_string(case.merge_sha)}",
        f"observed_in_diff_rule_ids = {_toml_array(sorted(set(rule_ids)))}",
        f"baseline_statuses = {_toml_array(statuses)}",
        'label = ""',
        "expected_rule_ids = []",
        'rationale = ""',
        "",
    ]


def render_worksheet(
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
    reviewer: str,
) -> str:
    if reviewer not in {"a", "b"}:
        raise HoldoutManifestError("reviewer must be 'a' or 'b'")
    corpus, protocol, cases = validate_manifest(manifest_path, rules_reference, zoo_manifest)
    if protocol != ANNOTATION_PROTOCOL:
        raise HoldoutManifestError(f"manifest protocol must be {ANNOTATION_PROTOCOL}")
    data = load_collection(collection_path, manifest_path, rules_reference, zoo_manifest)
    observations = _cases_by_id(data)
    if data["label_state"] != "pending" or any(case.label_state != "pending" for case in cases):
        raise HoldoutManifestError("annotation worksheets require a pending collection")
    lines = _worksheet_header(data, reviewer)
    for case in cases:
        if case.case_id not in observations:
            raise HoldoutManifestError(f"collection is missing case {case.case_id}")
        lines.extend(_worksheet_case(case, observations[case.case_id]))
    return "\n".join(lines)


def _load_toml(path: Path, artifact_name: str) -> dict[str, Any]:
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise HoldoutManifestError(f"cannot read {artifact_name} {path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError(f"{artifact_name} must be a TOML table")
    return data


def _validate_context(
    data: dict[str, Any],
    collection: dict[str, Any],
    cases: list[HoldoutCase],
    known_rules: set[str],
    reviewer: str | None,
) -> dict[str, dict[str, Any]]:
    if data.get("schema_version") != ANNOTATION_SCHEMA_VERSION:
        raise HoldoutManifestError("annotation schema_version is unsupported")
    for field in ("corpus", "protocol", "manifest_sha256", "collection_sha256"):
        if data.get(field) != collection.get(field, collection.get("manifest_sha256")):
            expected = collection.get(field)
            raise HoldoutManifestError(f"annotation {field} does not match collection ({expected})")
    actual_reviewer = data.get("reviewer")
    if actual_reviewer not in {"a", "b"} or (reviewer is not None and actual_reviewer != reviewer):
        raise HoldoutManifestError("annotation reviewer must match the requested independent reviewer")
    raw_cases = data.get("case")
    if not isinstance(raw_cases, list):
        raise HoldoutManifestError("annotation cases must be an array")
    expected = {case.case_id: case for case in cases}
    collected = _cases_by_id(collection)
    observed: dict[str, dict[str, Any]] = {}
    for raw in raw_cases:
        if not isinstance(raw, dict):
            raise HoldoutManifestError("annotation case must be a table")
        unknown = set(raw) - ANNOTATION_FIELDS
        if unknown:
            raise HoldoutManifestError(f"annotation case has unknown fields: {', '.join(sorted(unknown))}")
        case_id = raw.get("id")
        if not isinstance(case_id, str) or case_id in observed or case_id not in expected:
            raise HoldoutManifestError(f"annotation has unknown or duplicate case {case_id!r}")
        case = expected[case_id]
        for field in ("repo", "base_sha", "head_sha", "merge_sha"):
            if raw.get(field) != getattr(case, field):
                raise HoldoutManifestError(f"annotation case {case_id}: {field} does not match manifest")
        if raw.get("pull_request") != case.pull_request:
            raise HoldoutManifestError(f"annotation case {case_id}: pull_request does not match manifest")
        if not isinstance(raw.get("observed_in_diff_rule_ids"), list) or not all(
            isinstance(item, str) for item in raw["observed_in_diff_rule_ids"]
        ):
            raise HoldoutManifestError(f"annotation case {case_id}: invalid observed rule IDs")
        if not isinstance(raw.get("baseline_statuses"), list) or not all(
            isinstance(item, str) for item in raw["baseline_statuses"]
        ):
            raise HoldoutManifestError(f"annotation case {case_id}: invalid baseline statuses")
        observation = collected.get(case_id)
        if observation is None:
            raise HoldoutManifestError(f"collection is missing case {case_id}")
        review = observation.get("review")
        expected_rules = sorted(set(review.get("in_diff_rule_ids", []))) if isinstance(review, dict) else []
        if raw["observed_in_diff_rule_ids"] != expected_rules:
            raise HoldoutManifestError(f"annotation case {case_id}: observed rule evidence drifted")
        expected_statuses = [
            f"{baseline_id}={observation['baselines'][baseline_id]['status']}"
            for baseline_id in case.baseline_ids
        ]
        if raw["baseline_statuses"] != expected_statuses:
            raise HoldoutManifestError(f"annotation case {case_id}: baseline evidence drifted")
        if not isinstance(raw.get("label"), str) or raw["label"] not in ALLOWED_LABELS:
            raise HoldoutManifestError(f"annotation case {case_id}: label must be one of {sorted(ALLOWED_LABELS)}")
        if not isinstance(raw.get("expected_rule_ids"), list) or not all(
            isinstance(item, str) for item in raw["expected_rule_ids"]
        ):
            raise HoldoutManifestError(f"annotation case {case_id}: expected_rule_ids must be a string array")
        if any(rule_id not in known_rules for rule_id in raw["expected_rule_ids"]):
            raise HoldoutManifestError(f"annotation case {case_id}: expected_rule_ids contains an unknown rule")
        if raw["label"] == "defect-present" and not raw["expected_rule_ids"]:
            raise HoldoutManifestError(f"annotation case {case_id}: defect-present needs expected_rule_ids")
        if not isinstance(raw.get("rationale"), str) or not raw["rationale"].strip():
            raise HoldoutManifestError(f"annotation case {case_id}: rationale is required")
        observed[case_id] = raw
    if set(observed) != set(expected):
        raise HoldoutManifestError(f"annotation is missing cases: {', '.join(sorted(set(expected) - set(observed)))}")
    return observed


def load_annotation(
    path: Path,
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
    reviewer: str | None = None,
) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    collection = load_collection(collection_path, manifest_path, rules_reference, zoo_manifest)
    _, _, cases = validate_manifest(manifest_path, rules_reference, zoo_manifest)
    data = _load_toml(path, "annotation")
    known_rules = set(re.findall(r"^### `([^`]+)`", rules_reference.read_text(encoding="utf-8"), re.MULTILINE))
    return data, _validate_context(data, collection, cases, known_rules, reviewer)

