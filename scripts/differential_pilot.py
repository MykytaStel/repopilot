"""Render and validate one-reviewer exploratory differential assessments."""

from __future__ import annotations

import hashlib
import json
import re
import tomllib
from pathlib import Path
from typing import Any

from differential_artifact import validate_artifact
from real_history_contract import ALLOWED_LABELS, HoldoutManifestError, validate_manifest


PILOT_SCHEMA_VERSION = 1
PILOT_PROTOCOL = "single-expert-pilot-v1"
PILOT_MODE = "single-expert-exploratory"
PILOT_FIELDS = {
    "id",
    "repo",
    "pull_request",
    "base_sha",
    "head_sha",
    "merge_sha",
    "baseline_statuses",
    "label",
    "expected_rule_ids",
    "rationale",
}


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _toml_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def _toml_array(values: list[str]) -> str:
    return "[" + ", ".join(_toml_string(value) for value in values) + "]"


def _load_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HoldoutManifestError(f"cannot read differential artifact {path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError("differential artifact must be a JSON object")
    return data


def _cases_by_id(data: dict[str, Any]) -> dict[str, dict[str, Any]]:
    cases = data.get("cases")
    if not isinstance(cases, list):
        raise HoldoutManifestError("differential artifact cases must be an array")
    return {
        case["id"]: case
        for case in cases
        if isinstance(case, dict) and isinstance(case.get("id"), str)
    }


def _baseline_statuses(case: Any, observation: dict[str, Any]) -> list[str]:
    baselines = observation.get("baselines")
    if not isinstance(baselines, dict):
        raise HoldoutManifestError(f"pilot case {case.case_id}: baseline observations are missing")
    statuses = []
    for baseline_id in case.baseline_ids:
        runs = baselines.get(baseline_id)
        if not isinstance(runs, list) or not runs or not isinstance(runs[0], dict):
            raise HoldoutManifestError(f"pilot case {case.case_id}: baseline observations are incomplete")
        statuses.append(f"{baseline_id}={runs[0].get('status', 'unknown')}")
    return statuses


def render_pilot_template(
    artifact_path: Path,
    manifest_path: Path,
    differential_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
    reviewer: str,
) -> str:
    if not reviewer.strip():
        raise HoldoutManifestError("pilot reviewer must not be empty")
    validate_artifact(artifact_path, manifest_path, differential_path, rules_reference, zoo_manifest)
    artifact = _load_json(artifact_path)
    corpus, _, cases = validate_manifest(manifest_path, rules_reference, zoo_manifest)
    observations = _cases_by_id(artifact)
    lines = [
        f"schema_version = {PILOT_SCHEMA_VERSION}",
        f"corpus = {_toml_string(corpus)}",
        f"protocol = {_toml_string(PILOT_PROTOCOL)}",
        f"reviewer = {_toml_string(reviewer)}",
        f"manifest_sha256 = {_toml_string(sha256_file(manifest_path))}",
        f"differential_artifact_sha256 = {_toml_string(sha256_file(artifact_path))}",
        "blinded = true",
        f"assessment_mode = {_toml_string(PILOT_MODE)}",
        "",
    ]
    for case in cases:
        observation = observations.get(case.case_id)
        if observation is None:
            raise HoldoutManifestError(f"differential artifact is missing case {case.case_id}")
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
                'label = ""',
                "expected_rule_ids = []",
                'rationale = ""',
                "",
            ]
        )
    return "\n".join(lines)


def _load_toml(path: Path) -> dict[str, Any]:
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise HoldoutManifestError(f"cannot read pilot worksheet {path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError("pilot worksheet must be a TOML table")
    return data


def _known_rules(rules_reference: Path) -> set[str]:
    return set(re.findall(r"^### `([^`]+)`", rules_reference.read_text(encoding="utf-8"), re.MULTILINE))


def _validate_cases(
    data: dict[str, Any],
    artifact: dict[str, Any],
    cases: list[Any],
    known_rules: set[str],
) -> dict[str, dict[str, Any]]:
    raw_cases = data.get("case")
    if not isinstance(raw_cases, list):
        raise HoldoutManifestError("pilot worksheet cases must be an array")
    expected = {case.case_id: case for case in cases}
    observations = _cases_by_id(artifact)
    observed: dict[str, dict[str, Any]] = {}
    for raw in raw_cases:
        if not isinstance(raw, dict):
            raise HoldoutManifestError("pilot case must be a table")
        unknown = set(raw) - PILOT_FIELDS
        if unknown:
            raise HoldoutManifestError(f"pilot case has unknown fields: {', '.join(sorted(unknown))}")
        case_id = raw.get("id")
        if not isinstance(case_id, str) or case_id in observed or case_id not in expected:
            raise HoldoutManifestError(f"pilot has unknown or duplicate case {case_id!r}")
        case = expected[case_id]
        for field in ("repo", "base_sha", "head_sha", "merge_sha"):
            if raw.get(field) != getattr(case, field):
                raise HoldoutManifestError(f"pilot case {case_id}: {field} does not match manifest")
        if raw.get("pull_request") != case.pull_request:
            raise HoldoutManifestError(f"pilot case {case_id}: pull_request does not match manifest")
        observation = observations.get(case_id)
        if observation is None:
            raise HoldoutManifestError(f"differential artifact is missing case {case_id}")
        if raw.get("baseline_statuses") != _baseline_statuses(case, observation):
            raise HoldoutManifestError(f"pilot case {case_id}: baseline evidence drifted")
        label = raw.get("label")
        if label not in ALLOWED_LABELS:
            raise HoldoutManifestError(f"pilot case {case_id}: label must be one of {sorted(ALLOWED_LABELS)}")
        rule_ids = raw.get("expected_rule_ids")
        if not isinstance(rule_ids, list) or not all(isinstance(item, str) for item in rule_ids):
            raise HoldoutManifestError(f"pilot case {case_id}: expected_rule_ids must be a string array")
        if any(rule_id not in known_rules for rule_id in rule_ids):
            raise HoldoutManifestError(f"pilot case {case_id}: unknown rule in expected_rule_ids")
        if label == "defect-present" and not rule_ids:
            raise HoldoutManifestError(f"pilot case {case_id}: defect-present needs expected_rule_ids")
        if label != "defect-present" and rule_ids:
            raise HoldoutManifestError(f"pilot case {case_id}: non-positive label cannot name expected rules")
        rationale = raw.get("rationale")
        if not isinstance(rationale, str) or not rationale.strip():
            raise HoldoutManifestError(f"pilot case {case_id}: rationale is required")
        observed[case_id] = raw
    if set(observed) != set(expected):
        missing = ", ".join(sorted(set(expected) - set(observed)))
        raise HoldoutManifestError(f"pilot is missing cases: {missing}")
    return observed


def validate_pilot(
    pilot_path: Path,
    artifact_path: Path,
    manifest_path: Path,
    differential_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    validate_artifact(artifact_path, manifest_path, differential_path, rules_reference, zoo_manifest)
    artifact = _load_json(artifact_path)
    data = _load_toml(pilot_path)
    corpus, _, cases = validate_manifest(manifest_path, rules_reference, zoo_manifest)
    if data.get("schema_version") != PILOT_SCHEMA_VERSION or data.get("protocol") != PILOT_PROTOCOL:
        raise HoldoutManifestError("pilot schema or protocol is unsupported")
    if data.get("corpus") != corpus or data.get("blinded") is not True:
        raise HoldoutManifestError("pilot corpus or blinded marker is invalid")
    if data.get("assessment_mode") != PILOT_MODE:
        raise HoldoutManifestError("pilot assessment_mode must be single-expert-exploratory")
    if not isinstance(data.get("reviewer"), str) or not data["reviewer"].strip():
        raise HoldoutManifestError("pilot reviewer is required")
    if data.get("manifest_sha256") != sha256_file(manifest_path):
        raise HoldoutManifestError("pilot manifest_sha256 does not match manifest")
    if data.get("differential_artifact_sha256") != sha256_file(artifact_path):
        raise HoldoutManifestError("pilot differential_artifact_sha256 does not match artifact")
    validated = _validate_cases(data, artifact, cases, _known_rules(rules_reference))
    return {"status": "valid", "protocol": PILOT_PROTOCOL, "reviewer": data["reviewer"], "cases": len(validated)}


def load_pilot(
    pilot_path: Path,
    artifact_path: Path,
    manifest_path: Path,
    differential_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    validate_pilot(pilot_path, artifact_path, manifest_path, differential_path, rules_reference, zoo_manifest)
    data = _load_toml(pilot_path)
    raw_cases = data.get("case")
    if not isinstance(raw_cases, list):
        raise HoldoutManifestError("pilot worksheet cases must be an array")
    return data, {
        case["id"]: case
        for case in raw_cases
        if isinstance(case, dict) and isinstance(case.get("id"), str)
    }
