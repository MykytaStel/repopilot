"""Render deterministic Markdown for a label coverage audit."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from real_history_contract import HoldoutManifestError


def _ids(values: list[str]) -> str:
    return ", ".join(f"`{value}`" for value in values) if values else "—"


def render_label_coverage_report(data: dict[str, Any]) -> str:
    lines = [
        "# Real-history label coverage audit",
        "",
        f"- **Corpus:** {data.get('corpus', 'unknown')}",
        f"- **Label source:** {data.get('label_source', 'unknown')}",
        f"- **Cases:** {data.get('cases_labeled', 0)}/{data.get('cases_total', 0)} labeled",
        f"- **Dual metrics ready:** {'yes' if data.get('ready_for_dual_metrics') else 'no'}",
        f"- **Limitation:** {data.get('limitation', 'unspecified')}",
        "",
        "## Contract coverage",
        "",
        "| Contract ID | Measured | Observed cases | Observed in labeled cases | Expected cases | Unreviewed cases |",
        "| --- | :---: | ---: | ---: | ---: | ---: |",
    ]
    for contract_id in sorted(data.get("contract_coverage", {})):
        item = data["contract_coverage"][contract_id]
        lines.append(
            f"| `{contract_id}` | {'yes' if item['measured'] else 'no'} | "
            f"{item['observed_cases']} | {item['observed_labeled_cases']} | "
            f"{item['expected_cases']} | {item['unreviewed_cases']} |"
        )
    lines.extend(
        [
            "",
            "## Case gaps",
            "",
            "| Case | Label state | Observed without expected | Expected without observation | Unmeasured observed |",
            "| --- | --- | --- | --- | --- |",
        ]
    )
    for row in data.get("cases", []):
        lines.append(
            f"| `{row['id']}` | {row['label_state']} | {_ids(row['observed_without_expected_ids'])} | "
            f"{_ids(row['expected_without_observation_ids'])} | {_ids(row['unmeasured_observed_contract_ids'])} |"
        )
    lines.extend(
        [
            "",
            "This report audits packet completeness and unresolved observations. "
            "It does not infer label correctness or broaden the measured contract registry.",
            "",
        ]
    )
    return "\n".join(lines)


def write_label_coverage_report(coverage_path: Path, output_path: Path) -> str:
    try:
        data = json.loads(coverage_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HoldoutManifestError(f"cannot read label coverage artifact {coverage_path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError("label coverage artifact must be a JSON object")
    report = render_label_coverage_report(data)
    output_path.write_text(report, encoding="utf-8")
    return report
