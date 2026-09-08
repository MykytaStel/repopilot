"""Evaluate the preregistered cold/warm differential resource budget."""

from __future__ import annotations

import statistics
import tomllib
from pathlib import Path
from typing import Any

from differential_manifest import DifferentialManifestError
from differential_budget_workload import (
    WorkloadPolicyError,
    normalize_verification_checks,
    workload_mismatch,
)


REQUIRED_PHASES = ("cold", "warm")
POLICY_SCHEMA_VERSION = 1


class DifferentialBudgetError(DifferentialManifestError):
    """Raised when a resource policy or artifact cannot be enforced."""


def load_manifest_document(manifest_path: Path) -> dict[str, Any]:
    try:
        document = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise DifferentialBudgetError(f"cannot read differential manifest {manifest_path}: {error}") from error
    return document


def _positive_number(value: object, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or value <= 0:
        raise DifferentialBudgetError(f"{label} must be a positive number")
    return float(value)


def _validate_policy(policy: object) -> dict[str, Any]:
    if not isinstance(policy, dict):
        raise DifferentialBudgetError("resource_policy must be a table")
    if policy.get("schema_version") != POLICY_SCHEMA_VERSION:
        raise DifferentialBudgetError(f"resource policy schema_version must be {POLICY_SCHEMA_VERSION}")
    for key in ("policy_id", "workload", "source", "unit", "statistic"):
        if not isinstance(policy.get(key), str) or not policy[key].strip():
            raise DifferentialBudgetError(f"resource policy {key} must be a non-empty string")
    phases = policy.get("required_phases")
    if not isinstance(phases, list) or set(phases) != set(REQUIRED_PHASES) or len(phases) != len(REQUIRED_PHASES):
        raise DifferentialBudgetError("resource policy required phases must be cold and warm")
    if policy.get("unit") != "KiB" or policy.get("statistic") != "median":
        raise DifferentialBudgetError("resource policy must use median KiB measurements")
    if policy.get("unavailable") != "fail":
        raise DifferentialBudgetError('resource policy unavailable must be "fail"')
    try:
        verification_checks = normalize_verification_checks(
            policy.get("verification_checks", []), "resource policy verification_checks"
        )
    except WorkloadPolicyError as error:
        raise DifferentialBudgetError(str(error)) from error
    ceilings = policy.get("ceiling_kb_by_phase")
    if not isinstance(ceilings, dict) or set(ceilings) != set(REQUIRED_PHASES):
        raise DifferentialBudgetError("resource policy must define cold and warm ceilings")
    normalized = dict(policy)
    normalized["required_phases"] = list(REQUIRED_PHASES)
    normalized["ceiling_kb_by_phase"] = {
        phase: _positive_number(ceilings[phase], f"{phase} ceiling") for phase in REQUIRED_PHASES
    }
    case_ceilings = policy.get("case_ceiling_kb_by_phase", {})
    if not isinstance(case_ceilings, dict):
        raise DifferentialBudgetError("case_ceiling_kb_by_phase must be a table")
    normalized_cases: dict[str, dict[str, float]] = {}
    for case_id, values in case_ceilings.items():
        if not isinstance(case_id, str) or not case_id.strip() or not isinstance(values, dict):
            raise DifferentialBudgetError("case resource ceilings must map IDs to phase tables")
        if set(values) != set(REQUIRED_PHASES):
            raise DifferentialBudgetError(f"case {case_id} must define cold and warm ceilings")
        normalized_cases[case_id] = {
            phase: _positive_number(values[phase], f"{case_id} {phase} ceiling")
            for phase in REQUIRED_PHASES
        }
    normalized["case_ceiling_kb_by_phase"] = normalized_cases
    normalized["verification_checks"] = sorted(verification_checks)
    return normalized


def load_resource_policy(manifest_path: Path) -> dict[str, Any] | None:
    document = load_manifest_document(manifest_path)
    raw_policy = document.get("resource_policy")
    return None if raw_policy is None else _validate_policy(raw_policy)


def _case_ids(manifest: dict[str, Any]) -> list[str]:
    raw_cases = manifest.get("case")
    if not isinstance(raw_cases, list) or not raw_cases:
        raise DifferentialBudgetError("resource budget manifest must contain cases")
    ids = [case.get("id") for case in raw_cases if isinstance(case, dict)]
    if len(ids) != len(raw_cases) or not all(isinstance(case_id, str) and case_id.strip() for case_id in ids):
        raise DifferentialBudgetError("resource budget manifest cases must have IDs")
    if len(set(ids)) != len(ids):
        raise DifferentialBudgetError("resource budget manifest case IDs must be unique")
    return ids


def _violation(kind: str, case: str, phase: str | None, message: str, **values: object) -> dict[str, object]:
    result: dict[str, object] = {"case": case, "phase": phase, "kind": kind, "message": message}
    result.update(values)
    return result


def _review_samples(case: dict[str, Any], policy: dict[str, Any], violations: list[dict[str, object]]) -> dict[str, list[float]]:
    samples = {phase: [] for phase in REQUIRED_PHASES}
    reviews = case.get("reviews")
    if not isinstance(reviews, list):
        violations.append(_violation("missing_reviews", case["id"], None, "case has no review runs"))
        return samples
    for run in reviews:
        if not isinstance(run, dict):
            violations.append(_violation("invalid_review", case["id"], None, "review run is not an object"))
            continue
        phase = run.get("resource_phase")
        phase_value = phase if isinstance(phase, str) else None
        if phase_value is None:
            violations.append(_violation("missing_phase", case["id"], None, "review run has no resource phase"))
        elif phase_value not in REQUIRED_PHASES:
            violations.append(_violation("invalid_phase", case["id"], phase_value, "resource phase is unsupported"))
        status = run.get("resource_status")
        if status != "available":
            reason = run.get("resource_reason", "resource sample is unavailable")
            kind = "unavailable" if status == "unavailable" else "invalid_status"
            violations.append(_violation(kind, case["id"], phase_value, str(reason)))
            continue
        source = run.get("resource_source")
        if source != policy["source"]:
            violations.append(
                _violation(
                    "source_mismatch",
                    case["id"],
                    phase_value,
                    f"resource source {source!r} does not match {policy['source']!r}",
                )
            )
            continue
        value = run.get("child_max_rss_kb")
        if phase_value not in samples or isinstance(value, bool) or not isinstance(value, (int, float)) or value <= 0:
            violations.append(_violation("invalid_sample", case["id"], phase_value, "RSS sample must be positive"))
            continue
        samples[phase_value].append(float(value))
    return samples


def _phase_summary(values: list[float], phase: str, ceiling: float) -> dict[str, object]:
    if not values:
        return {"status": "fail", "samples": 0, "median_kb": None, "max_kb": None, "ceiling_kb": ceiling}
    return {
        "status": "pass",
        "samples": len(values),
        "median_kb": round(statistics.median(values), 3),
        "max_kb": round(max(values), 3),
        "ceiling_kb": ceiling,
    }


def evaluate_resource_budget(
    artifact: dict[str, Any], manifest: dict[str, Any], policy: dict[str, Any]
) -> dict[str, object]:
    normalized = _validate_policy(policy)
    try:
        mismatch = workload_mismatch(normalized, artifact, REQUIRED_PHASES)
    except WorkloadPolicyError as error:
        raise DifferentialBudgetError(str(error)) from error
    if mismatch is not None:
        return mismatch
    manifest_ids = _case_ids(manifest)
    raw_cases = artifact.get("cases")
    if not isinstance(raw_cases, list):
        raise DifferentialBudgetError("artifact cases must be an array")
    artifact_ids = [case.get("id") for case in raw_cases if isinstance(case, dict)]
    if sorted(artifact_ids) != sorted(manifest_ids):
        raise DifferentialBudgetError("resource budget case set mismatch")
    unknown_case_limits = set(normalized["case_ceiling_kb_by_phase"]) - set(manifest_ids)
    if unknown_case_limits:
        raise DifferentialBudgetError("resource policy contains unknown case ceilings")
    violations: list[dict[str, object]] = []
    aggregate = {phase: [] for phase in REQUIRED_PHASES}
    case_summary: list[dict[str, object]] = []
    for case in sorted(raw_cases, key=lambda item: item["id"]):
        samples = _review_samples(case, normalized, violations)
        phase_summary: dict[str, object] = {}
        case_limits = normalized["case_ceiling_kb_by_phase"].get(case["id"], {})
        for phase in REQUIRED_PHASES:
            aggregate[phase].extend(samples[phase])
            ceiling = case_limits.get(phase, normalized["ceiling_kb_by_phase"][phase])
            summary = _phase_summary(samples[phase], phase, ceiling)
            phase_summary[phase] = summary
            if not samples[phase]:
                violations.append(_violation("missing_phase", case["id"], phase, "no valid RSS samples for phase"))
                continue
            if summary["median_kb"] > ceiling:
                violations.append(
                    _violation(
                        "median_over_ceiling",
                        case["id"],
                        phase,
                        "phase median exceeds ceiling",
                        observed_kb=summary["median_kb"],
                        ceiling_kb=ceiling,
                    )
                )
            if summary["max_kb"] > ceiling:
                violations.append(
                    _violation(
                        "max_over_ceiling",
                        case["id"],
                        phase,
                        "phase maximum exceeds ceiling",
                        observed_kb=summary["max_kb"],
                        ceiling_kb=ceiling,
                    )
                )
        case_summary.append({"id": case["id"], "phases": phase_summary})
    phase_summary = {}
    for phase in REQUIRED_PHASES:
        summary = _phase_summary(aggregate[phase], phase, normalized["ceiling_kb_by_phase"][phase])
        phase_summary[phase] = summary
        if not aggregate[phase]:
            violations.append(_violation("missing_phase", "aggregate", phase, "no valid RSS samples for phase"))
        elif summary["median_kb"] > summary["ceiling_kb"]:
            violations.append(
                _violation(
                    "median_over_ceiling",
                    "aggregate",
                    phase,
                    "aggregate phase median exceeds ceiling",
                    observed_kb=summary["median_kb"],
                    ceiling_kb=summary["ceiling_kb"],
                )
            )
        elif summary["max_kb"] > summary["ceiling_kb"]:
            violations.append(
                _violation(
                    "max_over_ceiling",
                    "aggregate",
                    phase,
                    "aggregate phase maximum exceeds ceiling",
                    observed_kb=summary["max_kb"],
                    ceiling_kb=summary["ceiling_kb"],
                )
            )
    violations.sort(key=lambda item: (str(item["case"]), str(item["phase"]), str(item["kind"]), str(item["message"])))
    return {
        "status": "fail" if violations else "pass",
        "policy_id": normalized["policy_id"],
        "workload": normalized["workload"],
        "source": normalized["source"],
        "unit": normalized["unit"],
        "statistic": normalized["statistic"],
        "required_phases": list(REQUIRED_PHASES),
        "case_count": len(manifest_ids),
        "phase_summary": phase_summary,
        "cases": case_summary,
        "violations": violations,
    }


def render_resource_budget(result: dict[str, object]) -> str:
    lines = [
        f"Differential resource budget: {result['status']}",
        f"Policy: {result['policy_id']} ({result['workload']})",
        f"Source: {result['source']}; unit: {result['unit']}; statistic: {result['statistic']}",
    ]
    for phase in REQUIRED_PHASES:
        summary = result["phase_summary"][phase]
        lines.append(
            f"{phase}: median={summary['median_kb']} KiB; max={summary['max_kb']} KiB; "
            f"ceiling={summary['ceiling_kb']} KiB; samples={summary['samples']}"
        )
    for item in result["violations"]:
        lines.append(f"violation: {item['case']}/{item['phase']} {item['kind']}: {item['message']}")
    return "\n".join(lines) + "\n"
