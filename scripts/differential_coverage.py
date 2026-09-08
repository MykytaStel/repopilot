"""Render an evidence coverage audit for a validated differential artifact."""

from __future__ import annotations

from typing import Any


_PYTEST_FAILURE_MAPPING_REASON = "python.tests review has no exact test-node failure identity"


def _number(value: Any) -> str:
    return "—" if value is None else str(value)


def _reasons(value: Any) -> str:
    if not isinstance(value, dict) or not value:
        return "—"
    return "; ".join(f"{reason} ({count})" for reason, count in sorted(value.items()))


def render_coverage_audit(validation: dict[str, Any]) -> str:
    """Render coverage denominators and explicit next actions, without scoring utility."""

    evidence = validation["baseline_evidence"]
    comparison = evidence["comparison"]
    verification = evidence.get("review_verification", {})
    lines = [
        "# Differential evidence coverage audit",
        "",
        f"- **Corpus:** {validation['corpus']}",
        f"- **Protocol:** {validation['protocol']}",
        f"- **Baseline evidence:** {evidence['measured']}/{evidence['tracked']} tracked runs measured",
        f"- **Review-comparable evidence:** {comparison['measured']} measured, "
        f"{comparison['unavailable']} unavailable, {comparison['untracked']} untracked",
        f"- **Explicit review verification:** {verification.get('measured', 0)} measured, "
        f"{verification.get('unavailable', 0)} unavailable, "
        f"{verification.get('untracked', 0)} untracked; "
        f"{verification.get('keys', 0)} exact keys",
        "",
        "This is an adapter-coverage audit. It does not estimate precision, recall, utility, or overlap "
        "when a baseline identity mapping is unavailable.",
        "",
        "## Coverage by baseline",
        "",
        "| Baseline | Runs | Comparable measured | Unavailable | Untracked | Keys | Rate | Reasons |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |",
    ]
    for baseline_id, details in comparison.get("by_baseline", {}).items():
        lines.append(
            f"| `{baseline_id}` | {details['total']} | {details['measured']} | "
            f"{details['unavailable']} | {details['untracked']} | {details['keys']} | "
            f"{_number(details['measurement_rate'])} | {_reasons(details['unavailable_reasons'])} |"
        )
    lines.extend(
        [
            "",
            "## Next measurement action",
            "",
        ]
    )
    reasons = comparison.get("unavailable_reasons", {})
    if reasons:
        if any(_PYTEST_FAILURE_MAPPING_REASON in reason for reason in reasons):
            if verification.get("measured", 0):
                lines.append(
                    "Static `python.tests` overlap remains unavailable. Explicit review verification "
                    "provides exact node evidence for the separate `review-verification-v1` path; "
                    "do not merge it into static review claims or derive overlap from a test path."
                )
            else:
                lines.append(
                    "For `python.tests`, keep node failures unavailable until review emits exact test-node "
                    "failure provenance. Do not derive overlap from a test path or changed test name."
                )
        else:
            lines.append(
                "Add or review an exact `review-exact-v1` identity adapter for the unavailable baseline "
                "reasons above before interpreting duplicate work. Keep those runs unavailable until the "
                "mapping is proven against changed-line provenance."
            )
    else:
        lines.append(
            "No unavailable comparison reasons were recorded; inspect the measured keys before scoring overlap."
        )
    lines.extend(["", "The report is deterministic and derived from a validated artifact.", ""])
    return "\n".join(lines)
