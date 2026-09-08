"""Keep differential RSS policies bound to the checks they measure."""

from __future__ import annotations

from typing import Any

from differential_manifest import DifferentialManifestError


class WorkloadPolicyError(DifferentialManifestError):
    """Raised when a resource policy and artifact describe different work."""


def normalize_verification_checks(value: object, label: str) -> list[str]:
    if (
        not isinstance(value, list)
        or not all(isinstance(check_id, str) and check_id.strip() for check_id in value)
        or len(set(value)) != len(value)
    ):
        raise WorkloadPolicyError(f"{label} must be unique non-empty strings")
    return sorted(value)


def artifact_verification_checks(artifact: dict[str, Any]) -> list[str]:
    verification = artifact.get("review_verification")
    if verification is None:
        return []
    if not isinstance(verification, dict):
        raise WorkloadPolicyError("artifact review_verification must be an object")
    return normalize_verification_checks(verification.get("checks", []), "artifact review_verification checks")


def workload_mismatch(
    policy: dict[str, Any], artifact: dict[str, Any], required_phases: tuple[str, ...]
) -> dict[str, object] | None:
    policy_checks = policy["verification_checks"]
    artifact_checks = artifact_verification_checks(artifact)
    if artifact_checks == policy_checks:
        return None
    phase_summary = {
        phase: {
            "status": "not_applicable",
            "samples": 0,
            "median_kb": None,
            "max_kb": None,
            "ceiling_kb": policy["ceiling_kb_by_phase"][phase],
        }
        for phase in required_phases
    }
    return {
        "status": "fail",
        "policy_id": policy["policy_id"],
        "workload": policy["workload"],
        "source": policy["source"],
        "unit": policy["unit"],
        "statistic": policy["statistic"],
        "required_phases": list(required_phases),
        "case_count": 0,
        "phase_summary": phase_summary,
        "cases": [],
        "violations": [
            {
                "case": "artifact",
                "phase": None,
                "kind": "workload_mismatch",
                "message": "resource policy verification checks do not match artifact workload",
                "policy_verification_checks": policy_checks,
                "artifact_verification_checks": artifact_checks,
            }
        ],
    }
