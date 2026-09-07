"""Bounded contract identities used by the real-history evidence protocol."""

from __future__ import annotations

import hashlib
import json
from typing import Any, Iterable


ALL_CONTRACT_IDS = (
    "public-symbol/removed-export",
    "dependency/added",
    "dependency/removed",
    "dependency/upgraded",
    "dependency/downgraded",
    "dependency/source-changed",
    "dependency/feature-changed",
    "dependency/alias-changed",
    "dependency/metadata-only",
    "delivery/trigger-changed",
    "delivery/permission-changed",
    "delivery/secret-use-changed",
    "delivery/action-reference-changed",
    "delivery/artifact-changed",
    "delivery/deployment-changed",
    "runtime-configuration/introduced",
    "runtime-configuration/removed",
    "runtime-configuration/renamed",
    "runtime-configuration/changed",
    "security-boundary/boundary-changed",
    "security-boundary/entrypoint-impacted",
    "test-coverage/test-changed",
    "test-coverage/test-missing",
)
CONTRACT_IDS = (
    "security-boundary/boundary-changed",
    "security-boundary/entrypoint-impacted",
    "test-coverage/test-changed",
    "test-coverage/test-missing",
)
_ALL_CONTRACT_ID_SET = set(ALL_CONTRACT_IDS)
_MEASURED_CONTRACT_ID_SET = set(CONTRACT_IDS)


class ContractEvidenceError(ValueError):
    """Raised when a contract observation or human label leaves the protocol."""


def _contract_id(value: Any, context: str) -> str:
    if not isinstance(value, dict):
        raise ContractEvidenceError(f"{context} must be an object")
    family = value.get("family")
    change = value.get("change")
    if not isinstance(family, str) or not family.strip() or not isinstance(change, str) or not change.strip():
        raise ContractEvidenceError(f"{context} needs non-empty family and change")
    identity = f"{family.strip()}/{change.strip()}"
    if identity not in _ALL_CONTRACT_ID_SET:
        raise ContractEvidenceError(f"{context}: unknown contract ID {identity!r}")
    return identity


def observed_contract_ids(report: dict[str, Any]) -> tuple[str, ...]:
    """Extract stable family/change IDs while discarding evidence prose and paths."""
    proof = report.get("change_proof")
    if proof is None:
        return ()
    if not isinstance(proof, dict):
        raise ContractEvidenceError("change_proof must be an object")
    raw = proof.get("contract_deltas", [])
    if not isinstance(raw, list):
        raise ContractEvidenceError("change_proof.contract_deltas must be an array")
    return tuple(sorted({_contract_id(delta, f"contract_deltas[{index}]") for index, delta in enumerate(raw)}))


def validate_expected_contract_ids(values: Iterable[Any], context: str = "expected_contract_ids") -> tuple[str, ...]:
    """Validate independent labels against the frozen contract registry."""
    if not isinstance(values, (list, tuple)) or not all(isinstance(value, str) for value in values):
        raise ContractEvidenceError(f"{context} must be a string array")
    normalized = tuple(sorted(value.strip() for value in values))
    if len(set(normalized)) != len(normalized):
        raise ContractEvidenceError(f"{context} contains duplicate contract IDs")
    unknown = sorted(set(normalized) - _MEASURED_CONTRACT_ID_SET)
    if unknown:
        raise ContractEvidenceError(f"{context}: unknown contract ID {unknown[0]!r}")
    return normalized


def validate_observed_contract_ids(values: Iterable[Any], context: str = "contract_delta_ids") -> tuple[str, ...]:
    """Validate collector output against every currently emitted contract identity."""
    if not isinstance(values, (list, tuple)) or not all(isinstance(value, str) for value in values):
        raise ContractEvidenceError(f"{context} must be a string array")
    normalized = tuple(sorted(value.strip() for value in values))
    if len(set(normalized)) != len(normalized):
        raise ContractEvidenceError(f"{context} contains duplicate contract IDs")
    unknown = sorted(set(normalized) - _ALL_CONTRACT_ID_SET)
    if unknown:
        raise ContractEvidenceError(f"{context}: unknown contract ID {unknown[0]!r}")
    return normalized


def contract_evidence_hash(contract_ids: Iterable[str]) -> str:
    """Hash the sorted stable IDs, excluding paths and explanatory evidence."""
    payload = json.dumps(sorted(set(contract_ids)), separators=(",", ":"))
    return hashlib.sha256(payload.encode()).hexdigest()
