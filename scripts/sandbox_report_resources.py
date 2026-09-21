"""Render bounded resource evidence for human-facing sandbox reports."""

from __future__ import annotations

import math
import statistics
from typing import Any, Iterable


def summarize_resources(resources: Iterable[Any]) -> str:
    """Summarize resource receipts without exposing command output or reasons."""

    values = list(resources)
    samples = [
        float(resource["peak_rss_kb"])
        for resource in values
        if isinstance(resource, dict)
        and resource.get("status") == "available"
        and _positive_finite(resource.get("peak_rss_kb"))
    ]
    sources = sorted(
        {
            resource.get("source")
            for resource in values
            if isinstance(resource, dict)
            and isinstance(resource.get("source"), str)
            and resource.get("source")
        }
    )
    if not samples:
        return f"unavailable; 0/{len(values)} available"
    source = ", ".join(sources) if sources else "source unavailable"
    return (
        f"median {statistics.median(samples):,.0f} KiB; "
        f"{len(samples)}/{len(values)} available; source {source}"
    )


def resource_section(summary: dict[str, Any]) -> list[str]:
    """Build a report section from pilot or mutation summary resource receipts."""

    rows: list[tuple[str, str]] = []
    for case in sorted(
        summary.get("cases", []), key=lambda item: str(item.get("case_id", ""))
    ):
        if summary.get("kind") == "pilot-summary":
            resources = [_pilot_resource(run) for run in case.get("runs", [])]
        else:
            analysis = case.get("analysis")
            resources = [analysis.get("resource")] if isinstance(analysis, dict) else []
        rows.append((str(case.get("case_id", "unknown")), summarize_resources(resources)))
    lines = [
        "## Resource evidence",
        "| Case | Analyze peak RSS |",
        "| --- | --- |",
    ]
    lines.extend(f"| `{case_id}` | {summary} |" for case_id, summary in rows)
    lines.extend(
        [
            "",
            "Resource summaries describe recorded child-process observations; unavailable samples are not treated as zero memory use.",
            "",
        ]
    )
    return lines


def _pilot_resource(run: Any) -> Any:
    """Read the receipt from the normalized pilot shape, with legacy fallback."""

    if not isinstance(run, dict):
        return None
    normalized = run.get("normalized")
    if isinstance(normalized, dict) and "resource" in normalized:
        return normalized.get("resource")
    return run.get("resource")


def _positive_finite(value: Any) -> bool:
    if not isinstance(value, (int, float)) or isinstance(value, bool):
        return False
    try:
        return math.isfinite(float(value)) and value > 0
    except (OverflowError, ValueError):
        return False
