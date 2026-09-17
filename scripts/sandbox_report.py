"""Render validated sandbox summaries as a human-readable local report."""

from __future__ import annotations

import re
from typing import Any

from sandbox_contract import SandboxManifestError


_SENSITIVE_WORD = re.compile(
    r"(?i)(secret|token|password|authorization|api[_-]?key|private[_-]?key)"
)
_STATUS_MEANING = {
    "passed": "The recorded checks matched their expected states.",
    "failed": "At least one recorded expectation did not match.",
    "drift": "Repeated normalized scan outputs differed, so reproducibility is not established.",
    "unavailable": "Required evidence was unavailable, so no quality claim is made.",
}


def _safe_text(value: Any) -> str:
    text = str(value)
    return _SENSITIVE_WORD.sub("[REDACTED]", text)[:240]


def _status(value: Any) -> str:
    return _safe_text(value or "unknown").upper()


def _reason(value: Any) -> str:
    return _safe_text(value) if value else "No additional reason recorded."


def _pilot_comparison(comparison: dict[str, Any]) -> str:
    state = _safe_text(comparison.get("status", "unknown"))
    details = [state]
    count = comparison.get("count")
    if isinstance(count, int) and not isinstance(count, bool):
        details.append(f"{count} normalized findings")
    digest = comparison.get("normalized_sha256")
    if isinstance(digest, str):
        details.append(f"hash {_safe_text(digest)}")
    reason = comparison.get("reason")
    if reason:
        details.append(_reason(reason))
    return "; ".join(details)


def _pilot_run_text(run: dict[str, Any]) -> str:
    phase = _safe_text(run.get("phase", "run"))
    run_status = _status(run.get("status"))
    oracle = _safe_text(run.get("oracle_status", "unknown"))
    normalized = run.get("normalized")
    scan = "normalized output unavailable"
    if isinstance(normalized, dict) and normalized.get("status") == "measured":
        count = normalized.get("count")
        scan = (
            f"{count} normalized findings"
            if isinstance(count, int) and not isinstance(count, bool)
            else "normalized output measured"
        )
    return f"{phase}={run_status} (oracle {oracle}, {scan})"


def _phase_chain(phases: dict[str, Any], expected_oracle: str) -> str:
    names = ("baseline", "mutate", "oracle", "revert")
    parts = []
    for name in names:
        phase = phases.get(name, {})
        value = (
            phase.get("status", "unavailable")
            if isinstance(phase, dict)
            else "unavailable"
        )
        suffix = " (expected)" if name == "oracle" and value == expected_oracle else ""
        parts.append(f"{name} `{_safe_text(value)}`{suffix}")
    return " → ".join(parts)


def _mutation_oracle(item: dict[str, Any]) -> str:
    actual = _safe_text(item.get("oracle_status", "unknown"))
    expected = _safe_text(item.get("expected_oracle", "unknown"))
    return (
        f"{actual} (expected)"
        if actual == expected
        else f"{actual} (expected {expected})"
    )


def _scan_text(analysis: Any) -> str:
    if not isinstance(analysis, dict) or analysis.get("status") != "measured":
        reason = analysis.get("reason") if isinstance(analysis, dict) else None
        return f"unavailable: {_reason(reason)}" if reason else "unavailable"
    count = analysis.get("count")
    label = (
        f"{count} normalized finding"
        if isinstance(count, int) and not isinstance(count, bool) and count == 1
        else f"{count} normalized findings"
        if isinstance(count, int) and not isinstance(count, bool)
        else "measured"
    )
    digest = analysis.get("sha256")
    return f"{label}; hash {_safe_text(digest)}" if isinstance(digest, str) else label


def _render_pilot_markdown(summary: dict[str, Any]) -> list[str]:
    repeats = summary.get("repeats", "unknown")
    policy = ", ".join(_safe_text(item) for item in summary.get("repeat_policy", []))
    lines = [
        "## Run setup",
        f"- Repetitions: `{_safe_text(repeats)} runs per case: {policy or 'policy unavailable'}`.",
        "- The first repetition is labelled `cold`; later repetitions are labelled `warm`.",
        "- Static analysis is summarized by normalized finding count and hash; raw output is omitted.",
        "",
        "## Cases",
        "| Case | Project | Analysis | Status | Oracle runs | Scan comparison |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for item in sorted(
        summary.get("cases", []), key=lambda case: str(case.get("case_id", ""))
    ):
        runs = item.get("runs", [])
        oracle_runs = ", ".join(
            _safe_text(run.get("oracle_status", "unknown")) for run in runs
        )
        comparison = item.get("comparison", {})
        lines.append(
            f"| `{_safe_text(item.get('case_id'))}` | `{_safe_text(item.get('project_id'))}` "
            f"| `{_safe_text(item.get('analysis_mode', 'default'))}` "
            f"| **{_status(item.get('status'))}** | `{oracle_runs or 'none'}` "
            f"| {_pilot_comparison(comparison)} |"
        )
    lines.extend(["", "## What happened"])
    for item in sorted(
        summary.get("cases", []), key=lambda case: str(case.get("case_id", ""))
    ):
        runs = "; ".join(_pilot_run_text(run) for run in item.get("runs", []))
        lines.append(
            f"- **{_safe_text(item.get('case_id'))}**: **{_status(item.get('status'))}**. "
            f"{runs or 'No run details recorded.'}"
        )
    lines.extend(
        [
            "",
            "## Limits",
            "- Repeat labels describe runner slots; they do not prove a shared analyzer cache or a cold operating-system cache.",
            "- This packet is technical reproducibility evidence. It does not measure production recall, precision, human usability, or independent expert approval.",
            "",
        ]
    )
    return lines


def _render_mutation_markdown(summary: dict[str, Any]) -> list[str]:
    lines = [
        "## Cases",
        "| Case | Project | Kind | Split | Analysis | Status | Oracle | RepoPilot scan | Lifecycle |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    cases = sorted(
        summary.get("cases", []), key=lambda case: str(case.get("case_id", ""))
    )
    for item in cases:
        kind = (
            "negative control"
            if item.get("mutation_kind") == "negative-control"
            else _safe_text(item.get("mutation_kind"))
        )
        lines.append(
            f"| `{_safe_text(item.get('case_id'))}` | `{_safe_text(item.get('project_id'))}` "
            f"| {kind} | `{_safe_text(item.get('split'))}` "
            f"| `{_safe_text(item.get('analysis_mode', 'default'))}` "
            f"| **{_status(item.get('status'))}** "
            f"| `{_mutation_oracle(item)}` | {_scan_text(item.get('analysis'))} | "
            f"{_phase_chain(item.get('phases', {}), str(item.get('expected_oracle', 'unknown')))} |"
        )
    lines.extend(
        [
            "",
            "## Interpretation",
            "- Overall `PASSED` means each mutation lifecycle matched its declared independent oracle. It does not mean RepoPilot detected every violation.",
            "- The RepoPilot scan column is a measured observation from the isolated mutated copy; it does not establish production precision, recall, or rule completeness.",
            "",
            "## What happened",
        ]
    )
    lines.append(
        "- A `violation` intentionally breaks a contract. Its independent oracle is expected to fail; that expected failure counts as a passing mutation case when setup and revert also pass."
    )
    lines.append(
        "- A `negative control` should remain valid, so its expected oracle state is `passed`."
    )
    for item in cases:
        lines.append(
            f"- **{_safe_text(item.get('case_id'))}**: **{_status(item.get('status'))}** in the `{_safe_text(item.get('split'))}` split. "
            f"RepoPilot scan: {_scan_text(item.get('analysis'))}. "
            f"Lifecycle: {_phase_chain(item.get('phases', {}), str(item.get('expected_oracle', 'unknown')))}."
        )
    lines.extend(
        [
            "",
            "## Limits",
            "- Mutation results show that the pinned mutation and independent oracle lifecycle behaved as declared; they are not production recall or precision measurements.",
            "- An `unavailable` phase means the case cannot support a quality conclusion and must be rerun or explicitly excluded.",
            "",
        ]
    )
    return lines


def _render_text(lines: list[str]) -> str:
    text_lines: list[str] = []
    for line in lines:
        if line.startswith("#"):
            text_lines.append(line.lstrip("# ").upper())
            text_lines.append("=" * len(text_lines[-1]))
        else:
            text_lines.append(line)
    return "\n".join(text_lines).rstrip() + "\n"


def render_report(summary: dict[str, Any], output_format: str = "markdown") -> str:
    """Render a summary that has already passed its matching validator."""
    kind = summary.get("kind")
    if kind not in {"pilot-summary", "mutation-summary"}:
        raise SandboxManifestError(
            "sandbox report supports pilot or mutation summaries"
        )
    title = "Technical pilot" if kind == "pilot-summary" else "Mutation packet"
    status = _safe_text(summary.get("status", "unknown"))
    lines = [
        "# Sandbox report",
        "",
        f"**{title}** · overall status: **{_status(status)}**",
        "",
        f"- Corpus: `{_safe_text(summary.get('corpus'))}`",
        f"- Manifest: `sha256:{_safe_text(summary.get('manifest_sha256'))}`",
        f"- Meaning: {_STATUS_MEANING.get(status, 'The summary status is not recognized.')}",
        f"- Recorded reason: {_reason(summary.get('reason'))}",
        "",
    ]
    lines.extend(
        _render_pilot_markdown(summary)
        if kind == "pilot-summary"
        else _render_mutation_markdown(summary)
    )
    lines.extend(
        [
            "## Next action",
            f"- {_next_action(summary)}",
            "",
        ]
    )
    if output_format == "text":
        return _render_text(lines)
    if output_format != "markdown":
        raise SandboxManifestError("sandbox report format must be markdown or text")
    return "\n".join(lines).rstrip() + "\n"


def _next_action(summary: dict[str, Any]) -> str:
    kind = str(summary.get("kind"))
    status = str(summary.get("status"))
    if status == "passed" and kind == "pilot-summary":
        return "Use this as technical reproducibility evidence; inspect the mutation packet and unresolved coverage before making a rule-quality claim."
    if status == "passed":
        violations = [
            item
            for item in summary.get("cases", [])
            if item.get("mutation_kind") == "violation"
        ]
        if violations and all(
            isinstance(item.get("analysis"), dict)
            and item["analysis"].get("status") == "measured"
            and item["analysis"].get("count") == 0
            for item in violations
        ):
            return "No RepoPilot findings were recorded for the violation cases. Decide whether that is expected rule scope or a missing signal before treating this packet as rule-quality evidence."
        return "Keep the evaluation cases frozen and add the next mutation pair only after its independent oracle is defined."
    if status == "drift":
        return "Inspect the per-run artifacts for differing normalized hashes, then fix or explain the environment before rerunning."
    if status == "unavailable":
        return "Resolve the unavailable scanner, image, oracle, or resource and rerun; do not infer a pass from missing evidence."
    return "Open the per-case artifacts, identify the first failed expectation, and rerun after correcting the pinned case or oracle."
