"""Statistical helpers for sandbox metrics."""

from __future__ import annotations

import math
import statistics
from typing import Any

from sandbox_contract import SandboxManifestError


def wilson_interval(
    successes: int, trials: int, z: float = 1.96
) -> tuple[float, float] | None:
    if trials < 0 or successes < 0 or successes > trials:
        raise SandboxManifestError("invalid sandbox metric counts")
    if trials == 0:
        return None
    proportion = successes / trials
    denominator = 1 + z * z / trials
    center = (proportion + z * z / (2 * trials)) / denominator
    margin = (
        z
        * math.sqrt(
            proportion * (1 - proportion) / trials + z * z / (4 * trials * trials)
        )
        / denominator
    )
    return max(0.0, center - margin), min(1.0, center + margin)


def metric(numerator: int, denominator: int) -> dict[str, Any]:
    interval = wilson_interval(numerator, denominator)
    return {
        "status": "measured" if denominator else "unavailable",
        "numerator": numerator,
        "denominator": denominator,
        "value": numerator / denominator if denominator else None,
        "wilson_95": list(interval) if interval else None,
        **({} if denominator else {"reason": "no eligible cases"}),
    }


def unavailable(reason: str) -> dict[str, Any]:
    return {
        "status": "unavailable",
        "numerator": None,
        "denominator": None,
        "value": None,
        "wilson_95": None,
        "reason": reason,
    }


def performance(cases: list[dict[str, Any]]) -> dict[str, Any]:
    walls = [
        value
        for case in cases
        for value in case.get("analyze_wall_ms_samples", [])
        if isinstance(value, (int, float))
    ]
    rss = [
        value
        for case in cases
        for value in case.get("peak_rss_kb_samples", [])
        if isinstance(value, (int, float))
    ]
    result: dict[str, Any] = {
        "analyze_wall_ms": {
            "status": "measured" if walls else "unavailable",
            "median_ms": round(statistics.median(walls), 3) if walls else None,
            "samples": len(walls),
        },
        "peak_rss_kb": {
            "status": "measured" if rss else "unavailable",
            "median_kb": round(statistics.median(rss), 3) if rss else None,
            "samples": len(rss),
        },
    }
    if not rss:
        result["peak_rss_kb"]["reason"] = "sandbox RSS sampler is unavailable"
    return result


def summary_metrics(
    summary: dict[str, Any], cases: list[dict[str, Any]]
) -> dict[str, Any]:
    total = len(cases)
    measured = [case for case in cases if case["analysis_status"] == "measured"]
    passed = [case for case in cases if case["status"] == "passed"]
    unavailable_cases = [case for case in cases if case["status"] == "unavailable"]
    metrics: dict[str, Any] = {
        "case_coverage": metric(len(measured), total),
        "lifecycle_pass_rate": metric(len(passed), total),
        "unavailable_rate": metric(len(unavailable_cases), total),
        "tp": unavailable(
            "expected rule IDs and baseline scan identities are not declared"
        ),
        "fn": unavailable(
            "expected rule IDs and baseline scan identities are not declared"
        ),
        "tn": unavailable(
            "expected rule IDs and baseline scan identities are not declared"
        ),
        "fp": unavailable(
            "expected rule IDs and baseline scan identities are not declared"
        ),
        "additional_value": unavailable(
            "summary has no baseline scan for exact comparison"
        ),
    }
    if summary.get("kind") == "pilot-summary":
        stable = [case for case in measured if case["status"] == "passed"]
        metrics["determinism_rate"] = metric(len(stable), len(measured))
        metrics["unresolved_rate"] = metric(
            sum(case["status"] == "drift" for case in cases), total
        )
    else:
        metrics["unresolved_rate"] = unavailable(
            "mutation summary has no explicit unresolved case status"
        )
        violations = [
            case for case in measured if case.get("mutation_kind") == "violation"
        ]
        negatives = [
            case for case in measured if case.get("mutation_kind") == "negative-control"
        ]
        declared_violations = [
            case for case in violations if case.get("expected_rule_ids")
        ]
        declared_negatives = [
            case for case in negatives if case.get("expected_rule_ids")
        ]
        metrics["violation_signal_rate"] = metric(
            sum(
                case.get("rule_observation", {}).get("status") == "matched"
                for case in declared_violations
            )
            if declared_violations
            else sum(
                isinstance(case.get("normalized_finding_count"), int)
                and case["normalized_finding_count"] > 0
                for case in violations
            ),
            len(declared_violations) if declared_violations else len(violations),
        )
        metrics["negative_control_clean_rate"] = metric(
            sum(
                case.get("rule_observation", {}).get("status") == "matched"
                for case in declared_negatives
            )
            if declared_negatives
            else sum(case.get("normalized_finding_count") == 0 for case in negatives),
            len(declared_negatives) if declared_negatives else len(negatives),
        )
        declared = [case for case in measured if case.get("expected_rule_ids")]
        metrics["expected_rule_match_rate"] = metric(
            sum(
                case.get("rule_observation", {}).get("status") == "matched"
                for case in declared
            ),
            len(declared),
        )
        declared_violations = [
            case for case in declared if case.get("mutation_kind") == "violation"
        ]
        declared_negatives = [
            case for case in declared if case.get("mutation_kind") == "negative-control"
        ]
        metrics["exact_violation_signal_rate"] = metric(
            sum(
                case.get("rule_observation", {}).get("status") == "matched"
                for case in declared_violations
            ),
            len(declared_violations),
        )
        metrics["exact_negative_control_clean_rate"] = metric(
            sum(
                case.get("rule_observation", {}).get("status") == "matched"
                for case in declared_negatives
            ),
            len(declared_negatives),
        )
        for split in ("tuning", "evaluation"):
            split_declared_violations = [
                case
                for case in declared_violations
                if case.get("split") == split
            ]
            split_declared_negatives = [
                case for case in declared_negatives if case.get("split") == split
            ]
            metrics[f"{split}_exact_violation_signal_rate"] = metric(
                sum(
                    case.get("rule_observation", {}).get("status") == "matched"
                    for case in split_declared_violations
                ),
                len(split_declared_violations),
            )
            metrics[f"{split}_exact_negative_control_clean_rate"] = metric(
                sum(
                    case.get("rule_observation", {}).get("status") == "matched"
                    for case in split_declared_negatives
                ),
                len(split_declared_negatives),
            )
    return metrics
