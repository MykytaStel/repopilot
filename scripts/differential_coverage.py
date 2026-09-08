"""Render an evidence coverage audit for a validated differential artifact."""

from __future__ import annotations

from typing import Any


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
    lines = [
        "# Differential evidence coverage audit",
        "",
        f"- **Corpus:** {validation['corpus']}",
        f"- **Protocol:** {validation['protocol']}",
        f"- **Baseline evidence:** {evidence['measured']}/{evidence['tracked']} tracked runs measured",
        f"- **Review-comparable evidence:** {comparison['measured']} measured, "
        f"{comparison['unavailable']} unavailable, {comparison['untracked']} untracked",
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
