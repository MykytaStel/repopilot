"""Render deterministic Markdown for a validated evidence metrics artifact."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def _metric_value(metrics: dict[str, Any], key: str) -> str:
    value = metrics.get(key, {}).get("value")
    return "—" if value is None else f"{value:.3f}"


def _counts_cells(counts: dict[str, Any]) -> str:
    return "{tp} | {fn} | {tn} | {fp} | {excluded}".format(
        tp=counts.get("tp", 0),
        fn=counts.get("fn", 0),
        tn=counts.get("tn", 0),
        fp=counts.get("fp", 0),
        excluded=counts.get("excluded", 0),
    )


def _counts_row(counts: dict[str, Any]) -> str:
    return f"| {_counts_cells(counts)} |"


def render_metrics_report(data: dict[str, Any]) -> str:
    counts = data.get("counts", {})
    lines = [
        "# Real-history evidence metrics",
        "",
        f"- **Corpus:** {data.get('corpus', 'unknown')}",
        f"- **Protocol:** {data.get('protocol', 'unknown')}",
        f"- **Scope:** {data.get('scope', 'unknown')}",
        f"- **Limitation:** {data.get('limitation', 'unspecified')}",
        "",
        "## Case outcomes",
        "",
        "| TP | FN | TN | FP | Excluded |",
        "| ---: | ---: | ---: | ---: | ---: |",
        _counts_row(counts),
        "",
    ]
    contract_counts = data.get("contract_counts", {})
    contract_metrics = data.get("contract_metrics", {})
    if contract_counts:
        lines.extend(
            [
                "## Contract outcomes",
                "",
                "| Contract ID | TP | FN | TN | FP | Excluded | Recall | Specificity | Precision |",
                "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
            ]
        )
        for contract_id in sorted(contract_counts):
            counts_for_id = contract_counts[contract_id]
            metrics_for_id = contract_metrics.get(contract_id, {})
            lines.append(
                f"| `{contract_id}` | {_counts_cells(counts_for_id)} | "
                f"{_metric_value(metrics_for_id, 'recall')} | "
                f"{_metric_value(metrics_for_id, 'specificity')} | "
                f"{_metric_value(metrics_for_id, 'precision')} |"
            )
        lines.append("")
    lines.extend(
        [
            "This report is a deterministic rendering of a metrics artifact. "
            "Run the corresponding validator before treating it as evidence.",
            "",
        ]
    )
    return "\n".join(lines)


def write_metrics_report(metrics_path: Path, output_path: Path) -> str:
    data = json.loads(metrics_path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError("metrics artifact must be a JSON object")
    report = render_metrics_report(data)
    output_path.write_text(report, encoding="utf-8")
    return report
