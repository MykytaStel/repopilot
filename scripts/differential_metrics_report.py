"""Render deterministic Markdown for a differential pilot metrics artifact."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from real_history_contract import HoldoutManifestError


def _number(value: Any) -> str:
    return "—" if value is None else f"{float(value):.3f}"


def _ids(values: list[str]) -> str:
    return ", ".join(f"`{value}`" for value in values) if values else "—"


def _measurement_status(value: Any) -> str:
    if not isinstance(value, dict):
        return "unavailable"
    if value.get("status") == "unavailable":
        return f"unavailable: {value.get('reason', 'unspecified')}"
    if value.get("status") == "measured":
        details = []
        if isinstance(value.get("median_ms"), (int, float)):
            details.append(f"median {float(value['median_ms']):.3f} ms")
        if isinstance(value.get("overlap_count"), int):
            details.append(f"overlap {value['overlap_count']}")
        if isinstance(value.get("overlap_rate"), (int, float)):
            details.append(f"rate {float(value['overlap_rate']):.3f}")
        return "measured" + (f" ({', '.join(details)})" if details else "")
    return str(value.get("status", "unknown"))


def render_differential_metrics_report(data: dict[str, Any]) -> str:
    counts = data.get("counts", {})
    measurements = data.get("measurements", {})
    lines = [
        "# Differential utility pilot metrics",
        "",
        f"- **Corpus:** {data.get('corpus', 'unknown')}",
        f"- **Protocol:** {data.get('protocol', 'unknown')}",
        f"- **Reviewer:** {data.get('reviewer', 'unknown')}",
        f"- **Scope:** {data.get('scope', 'unknown')}",
        f"- **Limitation:** {data.get('limitation', 'unspecified')}",
        "",
        "## Case outcomes",
        "",
        "| TP | FN | TN | FP | Excluded |",
        "| ---: | ---: | ---: | ---: | ---: |",
        f"| {counts.get('tp', 0)} | {counts.get('fn', 0)} | {counts.get('tn', 0)} | "
        f"{counts.get('fp', 0)} | {counts.get('excluded', 0)} |",
        "",
        "## Utility measurements",
        "",
        "| Measurement | Result |",
        "| --- | --- |",
        f"| Deterministic cases | {measurements.get('deterministic_cases', 0)}/{measurements.get('case_count', 0)} |",
        f"| Novel evidence cases | {measurements.get('novel_evidence_cases', 0)} |",
        f"| Median baseline wall time (ms) | {_number(measurements.get('median_baseline_wall_ms'))} |",
        f"| Median review wall time (ms) | {_number(measurements.get('median_review_wall_ms'))} |",
        f"| Median review child max RSS (KiB) | {_number(measurements.get('median_review_child_max_rss_kb'))} |",
        f"| Time to first useful evidence | {_measurement_status(measurements.get('time_to_first_useful_evidence'))} |",
        f"| Decision latency | {_measurement_status(measurements.get('decision_latency'))} |",
        f"| Duplicate work | {_measurement_status(measurements.get('duplicate_work'))} |",
        "",
        "## Case detail",
        "",
        "| Case | Outcome | Expected rules | Observed novel rules | Novel evidence | Review ms | RSS KiB |",
        "| --- | --- | --- | --- | ---: | ---: | ---: |",
    ]
    for case in data.get("cases", []):
        case_measurements = case.get("measurements", {})
        lines.append(
            f"| `{case.get('id', 'unknown')}` | {case.get('outcome', 'unknown')} | "
            f"{_ids(case.get('expected_rule_ids', []))} | {_ids(case.get('observed_novel_rule_ids', []))} | "
            f"{case.get('novel_evidence_count', 0)} | {_number(case_measurements.get('median_review_wall_ms'))} | "
            f"{_number(case_measurements.get('median_review_child_max_rss_kb'))} |"
        )
    lines.extend(
        [
            "",
            "This report is a deterministic rendering of a differential pilot metrics artifact. "
            "Validate the artifact before treating it as evidence; unavailable measurements remain unavailable.",
            "",
        ]
    )
    return "\n".join(lines)


def write_differential_metrics_report(metrics_path: Path, output_path: Path) -> str:
    try:
        data = json.loads(metrics_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HoldoutManifestError(f"cannot read differential pilot metrics {metrics_path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError("differential pilot metrics must be a JSON object")
    report = render_differential_metrics_report(data)
    output_path.write_text(report, encoding="utf-8")
    return report
