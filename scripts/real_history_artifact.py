"""Validate collected real-history benchmark artifacts against their manifest."""

from __future__ import annotations

import hashlib
import json
from collections import Counter
from pathlib import Path
from typing import Any

from real_history_contract import HoldoutCase, HoldoutManifestError, validate_manifest
from real_history_contracts import (
    ContractEvidenceError,
    contract_evidence_hash,
    validate_observed_contract_ids,
)
from real_history_runner import BASELINE_COMMANDS


COLLECTION_SCHEMA_VERSION = 2
BASELINE_STATUSES = {"passed", "failed", "unavailable", "timeout"}
REVIEW_STATUSES = {"collected", "timeout"}


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_collection_data(
    data: dict[str, Any],
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    corpus, protocol, cases = validate_manifest(manifest_path, rules_reference, zoo_manifest)
    if data.get("schema_version") != COLLECTION_SCHEMA_VERSION:
        raise HoldoutManifestError("collection schema_version is unsupported")
    if data.get("corpus") != corpus or data.get("protocol") != protocol:
        raise HoldoutManifestError("collection corpus/protocol does not match manifest")
    if data.get("manifest_sha256") != sha256_file(manifest_path):
        raise HoldoutManifestError("collection manifest_sha256 does not match manifest")
    scanner = data.get("scanner")
    if not isinstance(scanner, dict):
        raise HoldoutManifestError("collection is missing scanner provenance")
    for field in ("mode", "version", "report_schema_version", "workspace_version"):
        if not isinstance(scanner.get(field), str) or not scanner[field]:
            raise HoldoutManifestError(f"scanner provenance missing {field}")
    observations = data.get("cases")
    if not isinstance(observations, list):
        raise HoldoutManifestError("collection cases must be an array")
    expected = {case.case_id: case for case in cases}
    observed: set[str] = set()
    for observation in observations:
        if not isinstance(observation, dict):
            raise HoldoutManifestError("collection case must be an object")
        case_id = observation.get("id")
        if not isinstance(case_id, str) or case_id in observed or case_id not in expected:
            raise HoldoutManifestError(f"collection has unknown or duplicate case {case_id!r}")
        observed.add(case_id)
        validate_observation(observation, expected[case_id])
    if observed != set(expected):
        missing = sorted(set(expected) - observed)
        raise HoldoutManifestError(f"collection is missing cases: {', '.join(missing)}")
    if data.get("label_state") != "pending" and any(case.label_state == "pending" for case in cases):
        raise HoldoutManifestError("collection cannot advance label_state while manifest labels are pending")
    return {
        "corpus": corpus,
        "protocol": protocol,
        "cases": len(observations),
        "baseline_observations": sum(len(observation["baselines"]) for observation in observations),
        "review_observations": len(observations),
        "contract_id_counts": dict(
            sorted(
                Counter(
                    contract_id
                    for observation in observations
                    for contract_id in observation["review"]["contract_delta_ids"]
                ).items()
            )
        ),
        "status": "valid",
    }


def validate_observation(observation: dict[str, Any], case: HoldoutCase) -> None:
    for field in ("repo", "base_sha", "head_sha", "merge_sha"):
        if observation.get(field) != getattr(case, field):
            raise HoldoutManifestError(f"collection case {case.case_id}: {field} does not match manifest")
    if observation.get("pull_request") != case.pull_request:
        raise HoldoutManifestError(f"collection case {case.case_id}: pull_request does not match manifest")
    if observation.get("label_state") != case.label_state:
        raise HoldoutManifestError(f"collection case {case.case_id}: label_state does not match manifest")
    baselines = observation.get("baselines")
    if not isinstance(baselines, dict) or set(baselines) != set(case.baseline_ids):
        raise HoldoutManifestError(f"collection case {case.case_id}: baseline set does not match manifest")
    for baseline_id, result in baselines.items():
        if not isinstance(result, dict) or result.get("status") not in BASELINE_STATUSES:
            raise HoldoutManifestError(f"collection case {case.case_id}: invalid baseline result {baseline_id}")
        if result.get("command") != list(BASELINE_COMMANDS[baseline_id]):
            raise HoldoutManifestError(f"collection case {case.case_id}: baseline command drift for {baseline_id}")
    review = observation.get("review")
    if not isinstance(review, dict) or review.get("status") not in REVIEW_STATUSES:
        raise HoldoutManifestError(f"collection case {case.case_id}: invalid review result")
    try:
        contract_ids = validate_observed_contract_ids(
            review.get("contract_delta_ids"),
            f"collection case {case.case_id} contract_delta_ids",
        )
    except ContractEvidenceError as error:
        raise HoldoutManifestError(str(error)) from error
    if review.get("contract_delta_count") != len(contract_ids):
        raise HoldoutManifestError(f"collection case {case.case_id}: contract delta count drift")
    if review.get("contract_evidence_sha256") != contract_evidence_hash(contract_ids):
        raise HoldoutManifestError(f"collection case {case.case_id}: contract evidence hash drift")


def validate_collection_artifact(
    artifact_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    try:
        data = json.loads(artifact_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HoldoutManifestError(f"cannot read collection artifact {artifact_path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError("collection artifact must be a JSON object")
    return validate_collection_data(data, manifest_path, rules_reference, zoo_manifest)
