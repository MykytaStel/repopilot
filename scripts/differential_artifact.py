"""Validate repeated differential utility observations without scoring them."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

from differential_contract import validate_differential
from differential_manifest import DifferentialManifestError
from real_history_contract import HoldoutManifestError, validate_manifest
from real_history_runner import BASELINE_COMMANDS


REVIEW_STATUSES = {"collected", "timeout"}


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_data(
    data: dict[str, Any],
    manifest_path: Path,
    differential_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    protocol = validate_differential(differential_path, manifest_path, rules_reference, zoo_manifest)
    if data.get("schema_version") != 1:
        raise DifferentialManifestError("differential artifact schema_version is unsupported")
    if data.get("corpus") != protocol["corpus"] or data.get("protocol") != protocol["protocol"]:
        raise DifferentialManifestError("differential artifact corpus/protocol does not match manifest")
    if data.get("manifest_sha256") != sha256_file(manifest_path):
        raise DifferentialManifestError("differential artifact manifest_sha256 does not match manifest")
    if data.get("differential_manifest_sha256") != sha256_file(differential_path):
        raise DifferentialManifestError("differential artifact differential_manifest_sha256 does not match manifest")
    if data.get("repetitions") != protocol["repetitions"]:
        raise DifferentialManifestError("differential artifact repetitions do not match manifest")
    if data.get("label_state") != "pending":
        raise DifferentialManifestError("differential artifact must remain unlabeled")
    scanner = data.get("scanner")
    if not isinstance(scanner, dict):
        raise DifferentialManifestError("differential artifact is missing scanner provenance")
    for field in ("mode", "version", "report_schema_version", "workspace_version"):
        if not isinstance(scanner.get(field), str) or not scanner[field]:
            raise DifferentialManifestError(f"scanner provenance missing {field}")
    observations = data.get("cases")
    if not isinstance(observations, list):
        raise DifferentialManifestError("differential artifact cases must be an array")
    _, _, holdout_cases = validate_manifest(manifest_path, rules_reference, zoo_manifest)
    expected = {case.case_id: case for case in holdout_cases}
    differential_cases = {case.case_id: case for case in _load_cases(differential_path)}
    observed: set[str] = set()
    for observation in observations:
        if not isinstance(observation, dict):
            raise DifferentialManifestError("differential case must be an object")
        case_id = observation.get("id")
        if not isinstance(case_id, str) or case_id in observed or case_id not in expected:
            raise DifferentialManifestError(f"differential artifact has unknown or duplicate case {case_id!r}")
        observed.add(case_id)
        validate_case(observation, expected[case_id], differential_cases[case_id].baseline_ids, data["repetitions"])
    if observed != set(expected):
        raise DifferentialManifestError(f"differential artifact is missing cases: {', '.join(sorted(set(expected) - observed))}")
    return {
        "status": "valid",
        "corpus": protocol["corpus"],
        "protocol": protocol["protocol"],
        "cases": len(observations),
        "baseline_observations": sum(
            sum(len(runs) for runs in observation["baselines"].values())
            for observation in observations
        ),
        "review_observations": sum(len(observation["reviews"]) for observation in observations),
    }


def _load_cases(path: Path) -> list[Any]:
    from differential_manifest import load_differential

    return load_differential(path)[4]


def validate_case(
    observation: dict[str, Any],
    case: Any,
    baseline_ids: tuple[str, ...],
    repetitions: int,
) -> None:
    for field in ("repo", "base_sha", "head_sha", "merge_sha"):
        if observation.get(field) != getattr(case, field):
            raise DifferentialManifestError(f"case {case.case_id}: {field} does not match holdout manifest")
    baselines = observation.get("baselines")
    if not isinstance(baselines, dict) or set(baselines) != set(baseline_ids):
        raise DifferentialManifestError(f"case {case.case_id}: baseline set does not match differential manifest")
    for baseline_id, runs in baselines.items():
        if not isinstance(runs, list) or len(runs) != repetitions:
            raise DifferentialManifestError(f"case {case.case_id}: baseline {baseline_id} repetition count drift")
        for run in runs:
            if not isinstance(run, dict) or run.get("command") != list(BASELINE_COMMANDS[baseline_id]):
                raise DifferentialManifestError(f"case {case.case_id}: baseline command drift for {baseline_id}")
            if run.get("status") not in {"passed", "failed", "unavailable", "timeout"}:
                raise DifferentialManifestError(f"case {case.case_id}: invalid baseline status")
            if not isinstance(run.get("wall_ms"), (int, float)) or run["wall_ms"] < 0:
                raise DifferentialManifestError(f"case {case.case_id}: invalid baseline timing")
    reviews = observation.get("reviews")
    if not isinstance(reviews, list) or len(reviews) != repetitions:
        raise DifferentialManifestError(f"case {case.case_id}: review repetition count drift")
    for review in reviews:
        if not isinstance(review, dict) or review.get("status") not in REVIEW_STATUSES:
            raise DifferentialManifestError(f"case {case.case_id}: invalid review status")
        if not isinstance(review.get("wall_ms"), (int, float)) or review["wall_ms"] < 0:
            raise DifferentialManifestError(f"case {case.case_id}: invalid review timing")
        if review["status"] == "collected" and not isinstance(review.get("stable_evidence_sha256"), str):
            raise DifferentialManifestError(f"case {case.case_id}: collected review lacks stable evidence hash")
    determinism = observation.get("determinism")
    if not isinstance(determinism, dict) or not isinstance(determinism.get("stable_evidence_deterministic"), bool):
        raise DifferentialManifestError(f"case {case.case_id}: determinism summary is missing")


def validate_artifact(
    artifact_path: Path,
    manifest_path: Path,
    differential_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    try:
        data = json.loads(artifact_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HoldoutManifestError(f"cannot read differential artifact {artifact_path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError("differential artifact must be a JSON object")
    return validate_data(data, manifest_path, differential_path, rules_reference, zoo_manifest)
