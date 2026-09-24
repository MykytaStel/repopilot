"""Canonical semantic ChangeProof projection for benchmark comparison."""

from __future__ import annotations

import hashlib
import json
import re
from typing import Any

from changeproof_benchmark_contract import BenchmarkCase


PROOF_FIELDS = {"verdict", "reasons", "coverage", "obligations", "contracts", "capabilities"}
COVERAGE_FIELDS = {"scope", "requested_files", "analyzed_files", "excluded_files", "unsupported_files"}
OBLIGATION_FIELDS = {"applicable", "satisfied", "failed", "unavailable", "unselected", "stale"}
VERDICTS = {"BROKEN", "REVIEW", "VERIFIED", "NOT_ASSESSED"}
SCOPES = {"changed", "full"}
CAPABILITY_STATUSES = {"assessed", "limited", "unavailable"}
CONFIDENCES = {"high", "limited", None}
IDENTIFIER = re.compile(r"^[a-z0-9][a-z0-9._/-]*$")


class BenchmarkProofError(ValueError):
    """Raised when review JSON cannot support a semantic proof observation."""


def normalize_change_proof(proof: object) -> dict[str, object]:
    """Retain deterministic semantics, including counts and confidence, not prose/paths."""

    value = _object(proof, "change_proof")
    normalized = {
        "verdict": _enum(value.get("verdict"), VERDICTS, "change_proof.verdict"),
        "reasons": _records(_reason(item) for item in _list(value, "reasons")),
        "coverage": _coverage(_object(value.get("coverage"), "change_proof.coverage")),
        "obligations": _counts(_object(value.get("obligations"), "change_proof.obligations"), OBLIGATION_FIELDS, "obligations"),
        "contracts": _records(_contract(item) for item in _list(value, "contract_deltas")),
        "capabilities": _records(_capability(item) for item in _list(value, "capability_coverage")),
    }
    return validate_normalized_proof(normalized)


def validate_normalized_proof(proof: object) -> dict[str, object]:
    """Reject persisted proofs outside the exact path/snippet-free v1 projection."""

    value = _object(proof, "normalized proof")
    if set(value) != PROOF_FIELDS:
        raise BenchmarkProofError("normalized proof fields are invalid")
    return {
        "verdict": _enum(value.get("verdict"), VERDICTS, "normalized proof.verdict"),
        "reasons": _canonical_records(value.get("reasons"), _reason, "normalized proof.reasons"),
        "coverage": _coverage(_object(value.get("coverage"), "normalized proof.coverage")),
        "obligations": _counts(_object(value.get("obligations"), "normalized proof.obligations"), OBLIGATION_FIELDS, "obligations"),
        "contracts": _canonical_records(value.get("contracts"), _contract, "normalized proof.contracts"),
        "capabilities": _canonical_records(value.get("capabilities"), _capability, "normalized proof.capabilities"),
    }


def proof_semantic_sha256(proof: object) -> str:
    return normalized_proof_sha256(normalize_change_proof(proof))


def normalized_proof_sha256(proof: object) -> str:
    canonical = json.dumps(validate_normalized_proof(proof), sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def evaluate_case(case: BenchmarkCase, review: dict[str, object]) -> dict[str, object]:
    return evaluate_normalized_proof(case, normalize_change_proof(review.get("change_proof")))


def evaluate_normalized_proof(case: BenchmarkCase, proof: object) -> dict[str, object]:
    proof = validate_normalized_proof(proof)
    return {
        "decision": _compare_known(case.expected_verdict, proof["verdict"]),
        "reasons": _compare_state(case.reason_codes_state, case.reason_codes, [item["code"] for item in proof["reasons"]]),
        "contracts": _compare_state(case.contracts_state, case.contracts, [f"{item['family']}/{item['change']}" for item in proof["contracts"]]),
        "capabilities": _compare_state(case.capabilities_state, case.capabilities, [f"{item['id']}/{item['status']}" for item in proof["capabilities"]]),
        "coverage": _compare_state(case.coverage_state, case.coverage, proof["coverage"]),
        "obligations": _compare_state(case.obligations_state, case.obligations, proof["obligations"]),
        "claims": {"status": "unavailable", "reason": "v1 has no observed claims projection"},
    }


def _compare_known(expected: object, observed: object) -> dict[str, object]:
    return {"status": "pass" if expected == observed else "fail", "expected": expected, "observed": observed}


def _compare_state(state: str, expected: object, observed: object) -> dict[str, object]:
    if state == "known":
        return _compare_known(list(expected) if isinstance(expected, tuple) else expected, observed)
    if state == "known-empty":
        return _compare_known(type(observed)(), observed)
    return {"status": "unavailable", "reason": f"case state is {state}"}


def _reason(raw: object) -> dict[str, object]:
    value = _object(raw, "reason")
    return {"code": _identifier(value.get("code"), "reason.code"), "count": _count(value.get("count"), "reason.count")}


def _contract(raw: object) -> dict[str, object]:
    value = _object(raw, "contract delta")
    confidence = value.get("confidence")
    if confidence not in CONFIDENCES:
        raise BenchmarkProofError("contract confidence is invalid")
    return {"family": _identifier(value.get("family"), "contract family"), "change": _identifier(value.get("change"), "contract change"), "confidence": confidence}


def _capability(raw: object) -> dict[str, object]:
    value = _object(raw, "capability")
    return {"id": _identifier(value.get("id"), "capability id"), "status": _enum(value.get("status"), CAPABILITY_STATUSES, "capability status"), "count": _count(value.get("count"), "capability count")}


def _coverage(value: dict[str, object]) -> dict[str, object]:
    if set(value) != COVERAGE_FIELDS:
        raise BenchmarkProofError("coverage fields are invalid")
    return {"scope": _enum(value.get("scope"), SCOPES, "coverage.scope"), **_counts({key: value[key] for key in COVERAGE_FIELDS - {"scope"}}, COVERAGE_FIELDS - {"scope"}, "coverage")}


def _counts(value: dict[str, object], fields: set[str], context: str) -> dict[str, int]:
    if set(value) != fields:
        raise BenchmarkProofError(f"{context} fields are invalid")
    return {key: _count(value[key], f"{context}.{key}") for key in sorted(fields)}


def _records(records: Any) -> list[dict[str, object]]:
    return sorted(records, key=lambda item: json.dumps(item, sort_keys=True, separators=(",", ":")))


def _canonical_records(value: object, parser: Any, context: str) -> list[dict[str, object]]:
    if not isinstance(value, list):
        raise BenchmarkProofError(f"{context} must be an array")
    parsed = _records(parser(item) for item in value)
    if value != parsed or len({json.dumps(item, sort_keys=True) for item in parsed}) != len(parsed):
        raise BenchmarkProofError(f"{context} must be sorted and unique")
    return parsed


def _count(value: object, context: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        raise BenchmarkProofError(f"{context} must be a non-negative integer")
    return value


def _identifier(value: object, context: str) -> str:
    if not isinstance(value, str) or not IDENTIFIER.fullmatch(value):
        raise BenchmarkProofError(f"{context} must be a stable identifier")
    return value


def _enum(value: object, choices: set[object], context: str) -> str:
    if not isinstance(value, str) or value not in choices:
        raise BenchmarkProofError(f"{context} is invalid")
    return value


def _list(value: dict[str, object], field: str) -> list[object]:
    item = value.get(field)
    if not isinstance(item, list):
        raise BenchmarkProofError(f"change_proof.{field} must be an array")
    return item


def _object(value: object, context: str) -> dict[str, object]:
    if not isinstance(value, dict):
        raise BenchmarkProofError(f"{context} must be an object")
    return value
