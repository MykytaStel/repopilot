"""Generate, validate, and score a single-expert contract pilot."""

from __future__ import annotations

import hashlib
import json
import tomllib
from pathlib import Path
from typing import Any

from real_history_annotations import load_collection
from real_history_contract import HoldoutCase, HoldoutManifestError, validate_manifest
from real_history_contracts import ContractEvidenceError, validate_expected_contract_ids


PILOT_SCHEMA_VERSION = 1
PILOT_PROTOCOL = "single-expert-contract-pilot-v1"
PILOT_MODE = "single-expert-exploratory"
CONTRACT_LABELS = {"contract-present", "no-contract", "uncertain"}
PILOT_FIELDS = {
    "id",
    "repo",
    "pull_request",
    "base_sha",
    "head_sha",
    "merge_sha",
    "baseline_statuses",
    "contract_label",
    "expected_contract_ids",
    "rationale",
}


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _toml_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def _toml_array(values: list[str]) -> str:
    return "[" + ", ".join(_toml_string(value) for value in values) + "]"


def _load_toml(path: Path) -> dict[str, Any]:
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise HoldoutManifestError(f"cannot read contract pilot {path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError("contract pilot must be a TOML table")
    return data


def _cases_by_id(data: dict[str, Any]) -> dict[str, dict[str, Any]]:
    cases = data.get("cases")
    if not isinstance(cases, list):
        raise HoldoutManifestError("collection cases must be an array")
    return {
        case["id"]: case
        for case in cases
        if isinstance(case, dict) and isinstance(case.get("id"), str)
    }


def _baseline_statuses(case: HoldoutCase, observation: dict[str, Any]) -> list[str]:
    baselines = observation.get("baselines")
    if not isinstance(baselines, dict):
        raise HoldoutManifestError(f"pilot case {case.case_id}: baselines are missing")
    return [f"{baseline_id}={baselines[baseline_id]['status']}" for baseline_id in case.baseline_ids]


def render_contract_pilot_template(
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
    reviewer: str,
) -> str:
    if not reviewer.strip():
        raise HoldoutManifestError("contract pilot reviewer must not be empty")
    data = load_collection(collection_path, manifest_path, rules_reference, zoo_manifest)
    corpus, _, cases = validate_manifest(manifest_path, rules_reference, zoo_manifest)
    observations = _cases_by_id(data)
    if data["label_state"] != "pending" or any(case.label_state != "pending" for case in cases):
        raise HoldoutManifestError("contract pilot requires a pending collection")
    lines = [
        f"schema_version = {PILOT_SCHEMA_VERSION}",
        f"corpus = {_toml_string(corpus)}",
        f"protocol = {_toml_string(PILOT_PROTOCOL)}",
        f"reviewer = {_toml_string(reviewer)}",
        f"manifest_sha256 = {_toml_string(sha256_file(manifest_path))}",
        f"collection_sha256 = {_toml_string(data['collection_sha256'])}",
        "blinded = true",
        f"assessment_mode = {_toml_string(PILOT_MODE)}",
        "",
    ]
    for case in cases:
        observation = observations.get(case.case_id)
        if observation is None:
            raise HoldoutManifestError(f"collection is missing case {case.case_id}")
        lines.extend(
            [
                "[[case]]",
                f"id = {_toml_string(case.case_id)}",
                f"repo = {_toml_string(case.repo)}",
                f"pull_request = {case.pull_request}",
                f"base_sha = {_toml_string(case.base_sha)}",
                f"head_sha = {_toml_string(case.head_sha)}",
                f"merge_sha = {_toml_string(case.merge_sha)}",
                f"baseline_statuses = {_toml_array(_baseline_statuses(case, observation))}",
                'contract_label = ""',
                "expected_contract_ids = []",
                'rationale = ""',
                "",
            ]
        )
    return "\n".join(lines)


def _validate_case_context(
    data: dict[str, Any],
    collection: dict[str, Any],
    cases: list[HoldoutCase],
) -> dict[str, dict[str, Any]]:
    raw_cases = data.get("case")
    if not isinstance(raw_cases, list):
        raise HoldoutManifestError("contract pilot cases must be an array")
    expected = {case.case_id: case for case in cases}
    observations = _cases_by_id(collection)
    validated: dict[str, dict[str, Any]] = {}
    for raw in raw_cases:
        if not isinstance(raw, dict):
            raise HoldoutManifestError("contract pilot case must be a table")
        unknown = set(raw) - PILOT_FIELDS
        if unknown:
            raise HoldoutManifestError(f"contract pilot case has unknown fields: {', '.join(sorted(unknown))}")
        case_id = raw.get("id")
        if not isinstance(case_id, str) or case_id in validated or case_id not in expected:
            raise HoldoutManifestError(f"contract pilot has unknown or duplicate case {case_id!r}")
        case = expected[case_id]
        for field in ("repo", "base_sha", "head_sha", "merge_sha"):
            if raw.get(field) != getattr(case, field):
                raise HoldoutManifestError(f"contract pilot case {case_id}: {field} does not match manifest")
        if raw.get("pull_request") != case.pull_request:
            raise HoldoutManifestError(f"contract pilot case {case_id}: pull_request does not match manifest")
        observation = observations.get(case_id)
        if observation is None:
            raise HoldoutManifestError(f"collection is missing case {case_id}")
        statuses = raw.get("baseline_statuses")
        if statuses != _baseline_statuses(case, observation):
            raise HoldoutManifestError(f"contract pilot case {case_id}: baseline evidence drifted")
        label = raw.get("contract_label")
        if label not in CONTRACT_LABELS:
            raise HoldoutManifestError(
                f"contract pilot case {case_id}: contract_label must be one of {sorted(CONTRACT_LABELS)}"
            )
        try:
            expected_ids = validate_expected_contract_ids(
                raw.get("expected_contract_ids"),
                f"contract pilot case {case_id} expected_contract_ids",
            )
        except ContractEvidenceError as error:
            raise HoldoutManifestError(str(error)) from error
        if label == "contract-present" and not expected_ids:
            raise HoldoutManifestError(f"contract pilot case {case_id}: contract-present needs expected_contract_ids")
        if label != "contract-present" and expected_ids:
            raise HoldoutManifestError(f"contract pilot case {case_id}: only contract-present may name contracts")
        rationale = raw.get("rationale")
        if not isinstance(rationale, str) or not rationale.strip():
            raise HoldoutManifestError(f"contract pilot case {case_id}: rationale is required")
        validated[case_id] = raw
    if set(validated) != set(expected):
        missing = ", ".join(sorted(set(expected) - set(validated)))
        raise HoldoutManifestError(f"contract pilot is missing cases: {missing}")
    return validated


def validate_contract_pilot(
    pilot_path: Path,
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    collection = load_collection(collection_path, manifest_path, rules_reference, zoo_manifest)
    corpus, _, cases = validate_manifest(manifest_path, rules_reference, zoo_manifest)
    data = _load_toml(pilot_path)
    if data.get("schema_version") != PILOT_SCHEMA_VERSION or data.get("protocol") != PILOT_PROTOCOL:
        raise HoldoutManifestError("contract pilot schema or protocol is unsupported")
    if data.get("corpus") != corpus or data.get("blinded") is not True:
        raise HoldoutManifestError("contract pilot corpus or blinded marker is invalid")
    if data.get("assessment_mode") != PILOT_MODE:
        raise HoldoutManifestError("contract pilot assessment_mode must be single-expert-exploratory")
    if not isinstance(data.get("reviewer"), str) or not data["reviewer"].strip():
        raise HoldoutManifestError("contract pilot reviewer is required")
    if data.get("manifest_sha256") != sha256_file(manifest_path):
        raise HoldoutManifestError("contract pilot manifest_sha256 does not match manifest")
    if data.get("collection_sha256") != sha256_file(collection_path):
        raise HoldoutManifestError("contract pilot collection_sha256 does not match collection")
    validated = _validate_case_context(data, collection, cases)
    return {
        "status": "valid",
        "protocol": PILOT_PROTOCOL,
        "scope": "single-expert exploratory real-history contract pilot",
        "reviewer": data["reviewer"],
        "cases": len(validated),
    }

