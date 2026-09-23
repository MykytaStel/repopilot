"""Render deterministic Markdown for sandbox metrics."""

from __future__ import annotations

from typing import Any


def _metric_value(metric: dict[str, Any]) -> str:
    if metric.get("status") != "measured":
        return f"unavailable: {metric.get('reason', 'not measured')}"
    interval = metric.get("wilson_95")
    value = metric.get("value")
    if (
        isinstance(value, (int, float))
        and isinstance(interval, list)
        and len(interval) == 2
    ):
        return f"{value:.3f} ({interval[0]:.3f}–{interval[1]:.3f})"
    return "measured"


def _scan_label(count: Any) -> str:
    if not isinstance(count, int) or isinstance(count, bool):
        return "unavailable"
    noun = "finding" if count == 1 else "findings"
    return f"{count} normalized {noun}"


def _number(value: Any) -> Any:
    return "—" if value is None else value


def render_metrics_report(data: dict[str, Any]) -> str:
    metrics = data.get("metrics", {})
    lines = [
        "# Sandbox metrics",
        "",
        f"- **Corpus:** `{data.get('corpus', 'unknown')}`",
        f"- **Source:** `{data.get('source_summary_kind', 'unknown')}` ({data.get('source_status', 'unknown')})",
        f"- **Scope:** {data.get('scope', 'unknown')}",
        f"- **Limitation:** {data.get('limitation', 'unspecified')}",
        "",
        "## Metrics",
        "",
        "| Metric | Numerator | Denominator | Result |",
        "| --- | ---: | ---: | --- |",
    ]
    for name in sorted(metrics):
        metric = metrics[name]
        numerator = metric.get("numerator")
        denominator = metric.get("denominator")
        numerator = "—" if numerator is None else numerator
        denominator = "—" if denominator is None else denominator
        lines.append(
            f"| `{name}` | {numerator} | {denominator} | {_metric_value(metric)} |"
        )
    lines.extend(
        [
            "",
            "## Performance",
            "",
            "| Measurement | Result | Samples |",
            "| --- | --- | ---: |",
        ]
    )
    for name in ("analyze_wall_ms", "peak_rss_kb"):
        item = data.get("performance", {}).get(name, {})
        if item.get("status") == "measured":
            value = item.get("median_ms", item.get("median_kb"))
            unit = "ms" if name.endswith("wall_ms") else "KiB"
            result = f"median {value} {unit}"
        else:
            result = f"unavailable: {item.get('reason', 'not measured')}"
        lines.append(f"| `{name}` | {result} | {item.get('samples', 0)} |")
    lines.extend(
        [
            "",
            "## Case detail",
            "",
            "| Case | Split | Profile | Status | Rule evidence | Scan | Analyze ms | RSS KiB |",
            "| --- | --- | --- | --- | --- | --- | ---: | ---: |",
        ]
    )
    for case in sorted(
        data.get("cases", []), key=lambda item: str(item.get("case_id", ""))
    ):
        scan = _scan_label(case.get("normalized_finding_count"))
        rule_observation = case.get("rule_observation") or {}
        rule_status = rule_observation.get("status", "not-declared")
        lines.append(
            f"| `{case.get('case_id', 'unknown')}` | {case.get('split', 'pilot')} | "
            f"{case.get('profile', 'default')} | {case.get('status', 'unknown')} | "
            f"{rule_status} | {scan} | "
            f"{_number(case.get('analyze_wall_ms'))} | "
            f"{_number(case.get('peak_rss_kb'))} |"
        )
    lines.extend(
        [
            "",
            "Wilson intervals are 95% intervals for the recorded numerator/denominator only. "
            "Unavailable TP/FN/TN/FP and additional-value metrics require expected rule identities and a baseline scan.",
            "",
        ]
    )
    return "\n".join(lines)
