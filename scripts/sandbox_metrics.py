"""Compute recomputable metrics from validated sandbox summaries."""

from __future__ import annotations

import hashlib
import json
import statistics
from pathlib import Path
from typing import Any

from sandbox_contract import (
    SandboxManifestError,
    validate_artifact,
    validate_mutation_summary,
    validate_pilot_summary,
)
from sandbox_metrics_stats import performance, summary_metrics


METRICS_PROTOCOL = "repopilot-validation-sandbox-metrics-v1"
METRICS_SCHEMA_VERSION = 1


def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _analysis_sha256() -> str:
    digest = hashlib.sha256()
    for path in (Path(__file__), Path(__file__).with_name("sandbox_metrics_stats.py")):
        digest.update(path.read_bytes())
    return digest.hexdigest()


def _load_json(path: Path, label: str) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise SandboxManifestError(f"cannot read {label} {path}: {error}") from error
    if not isinstance(data, dict):
        raise SandboxManifestError(f"{label} must be a JSON object")
    return data


def _validate_summary(
    summary_path: Path, manifest_path: Path, summary: dict[str, Any]
) -> None:
    if summary.get("kind") == "pilot-summary":
        validate_pilot_summary(summary_path, manifest_path)
    elif summary.get("kind") == "mutation-summary":
        validate_mutation_summary(summary_path, manifest_path)
    else:
        raise SandboxManifestError("sandbox metrics requires pilot or mutation summary")


def _artifact_path(summary_path: Path, name: str) -> Path:
    candidate = (summary_path.parent / name).resolve()
    try:
        candidate.relative_to(summary_path.parent.resolve())
    except ValueError as error:
        raise SandboxManifestError(
            "sandbox metrics artifact path escapes summary directory"
        ) from error
    return candidate


def _phase(artifact: dict[str, Any], name: str) -> dict[str, Any]:
    return next(
        (phase for phase in artifact.get("phases", []) if phase.get("name") == name),
        {},
    )


def _artifact_measurement(
    artifact: dict[str, Any],
) -> tuple[str, int | None, str | None, float | None, float | None]:
    phase = _phase(artifact, "analyze")
    result = phase.get("result") if isinstance(phase, dict) else None
    normalized = result.get("normalized_findings") if isinstance(result, dict) else None
    if not isinstance(normalized, dict) or normalized.get("status") != "measured":
        return "unavailable", None, None, _wall_ms(result), _rss_kb(result)
    count = normalized.get("count")
    digest = normalized.get("sha256")
    if not isinstance(count, int) or isinstance(count, bool) or count < 0:
        return "unavailable", None, None, _wall_ms(result), _rss_kb(result)
    return (
        "measured",
        count,
        digest if isinstance(digest, str) else None,
        _wall_ms(result),
        _rss_kb(result),
    )


def _wall_ms(result: Any) -> float | None:
    value = result.get("wall_ms") if isinstance(result, dict) else None
    return (
        float(value)
        if isinstance(value, (int, float)) and not isinstance(value, bool)
        else None
    )


def _rss_kb(result: Any) -> float | None:
    resource = result.get("resource") if isinstance(result, dict) else None
    value = resource.get("peak_rss_kb") if isinstance(resource, dict) else None
    return (
        float(value)
        if isinstance(value, (int, float)) and not isinstance(value, bool)
        else None
    )


def _case_artifacts(
    summary: dict[str, Any], summary_path: Path, manifest_path: Path
) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for item in summary.get("cases", []):
        names = (
            [run["artifact"] for run in item.get("runs", [])]
            if summary.get("kind") == "pilot-summary"
            else [item["artifact"]]
        )
        measurements = []
        for name in names:
            artifact_path = _artifact_path(summary_path, name)
            validate_artifact(artifact_path, manifest_path)
            artifact = _load_json(artifact_path, "sandbox artifact")
            measurements.append(_artifact_measurement(artifact))
        expected = item.get("analysis")
        if summary.get("kind") == "mutation-summary" and isinstance(expected, dict):
            if any(
                measurement[0] != expected.get("status")
                or (
                    expected.get("count") is not None
                    and measurement[1] != expected["count"]
                )
                or (
                    expected.get("sha256") is not None
                    and measurement[2] != expected["sha256"]
                )
                for measurement in measurements
            ):
                raise SandboxManifestError(
                    f"sandbox artifact analysis does not match summary case {item.get('case_id')}"
                )
        measured = [
            measurement for measurement in measurements if measurement[0] == "measured"
        ]
        counts = [
            measurement[1] for measurement in measured if measurement[1] is not None
        ]
        walls = [
            measurement[3] for measurement in measurements if measurement[3] is not None
        ]
        rss = [
            measurement[4] for measurement in measurements if measurement[4] is not None
        ]
        records.append(
            {
                "case_id": item.get("case_id"),
                "status": item.get("status"),
                "mutation_kind": item.get("mutation_kind"),
                "split": item.get("split", "pilot"),
                "analysis_mode": item.get("analysis_mode", "default"),
                "profile": item.get("profile", "default"),
                "expected_rule_ids": item.get("expected_rule_ids", []),
                "observed_rule_ids": item.get("observed_rule_ids", []),
                "rule_observation": item.get("rule_observation"),
                "analysis_status": "measured"
                if len(measured) == len(measurements)
                else "unavailable",
                "normalized_finding_count": counts[0]
                if counts and len(set(counts)) == 1
                else None,
                "analyze_wall_ms": round(statistics.median(walls), 3)
                if walls
                else None,
                "analyze_wall_ms_samples": [round(value, 3) for value in walls],
                "peak_rss_kb": round(statistics.median(rss), 3) if rss else None,
                "peak_rss_kb_samples": [round(value, 3) for value in rss],
            }
        )
    return records


def build_metrics(summary_path: Path, manifest_path: Path) -> dict[str, Any]:
    summary = _load_json(summary_path, "sandbox summary")
    _validate_summary(summary_path, manifest_path, summary)
    cases = _case_artifacts(summary, summary_path, manifest_path)
    return {
        "schema_version": METRICS_SCHEMA_VERSION,
        "kind": "sandbox-metrics",
        "protocol": METRICS_PROTOCOL,
        "source_summary_kind": str(summary["kind"]),
        "corpus": summary.get("corpus"),
        "manifest_sha256": summary.get("manifest_sha256"),
        "summary_sha256": _sha256_file(summary_path),
        "analysis_sha256": _analysis_sha256(),
        "scope": "technical reproducibility and controlled mutation observations",
        "limitation": "descriptive pinned-corpus evidence; not production or language-wide precision/recall",
        "source_status": summary.get("status"),
        "metrics": summary_metrics(summary, cases),
        "performance": performance(cases),
        "cases": cases,
    }
