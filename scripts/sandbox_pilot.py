"""Sequential repeated pilot orchestration over sandbox manifest cases."""

from __future__ import annotations

import json
import uuid
from pathlib import Path
from typing import Any

from sandbox_case import _redact_command
from sandbox_contract import (
    PROTOCOL,
    SandboxManifest,
    SandboxManifestError,
    load_manifest,
)
from sandbox_runner import DockerAdapter, run_case


PILOT_SCHEMA_VERSION = 1


def _write_summary(path: Path, summary: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{uuid.uuid4().hex}.tmp")
    temporary.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    temporary.replace(path)


def _analyze_record(artifact: dict[str, Any]) -> dict[str, Any]:
    phase = next(
        (
            phase
            for phase in artifact.get("phases", [])
            if phase.get("name") == "analyze"
        ),
        None,
    )
    result = phase.get("result", {}) if isinstance(phase, dict) else {}
    normalized = (
        result.get("normalized_findings", {}) if isinstance(result, dict) else {}
    )
    if not isinstance(normalized, dict):
        normalized = {}
    return {
        "status": normalized.get("status", "unavailable"),
        "count": normalized.get("count"),
        "sha256": normalized.get("sha256"),
        "reason": normalized.get("reason")
        or (phase.get("reason") if isinstance(phase, dict) else None),
    }


def _case_comparison(runs: list[dict[str, Any]]) -> dict[str, Any]:
    measured = [run for run in runs if run["normalized"]["status"] == "measured"]
    hashes = {run["normalized"].get("sha256") for run in measured}
    if len(measured) == len(runs) and len(hashes) == 1:
        return {
            "status": "stable",
            "normalized_sha256": next(iter(hashes)),
            "count": measured[0]["normalized"].get("count"),
        }
    if len(hashes) > 1:
        return {
            "status": "drift",
            "normalized_sha256": sorted(
                hash_value for hash_value in hashes if hash_value
            ),
            "reason": "normalized findings differ across repeated runs",
        }
    reasons = sorted(
        {
            str(run["normalized"].get("reason") or "normalized output unavailable")
            for run in runs
            if run["normalized"]["status"] != "measured"
        }
    )
    return {
        "status": "unavailable",
        "reason": "; ".join(reasons) or "normalized output was not measured",
    }


def _case_status(comparison: dict[str, Any], runs: list[dict[str, Any]]) -> str:
    if any(run.get("status") != "passed" for run in runs):
        return "unavailable"
    if comparison["status"] == "drift":
        return "drift"
    if comparison["status"] == "stable":
        return "passed"
    return "unavailable"


def _case_summary(
    manifest: SandboxManifest,
    case_id: str,
    output_dir: Path,
    source_root: Path | None,
    scanner: tuple[str, ...] | None,
    repeats: int,
    docker: DockerAdapter | None,
    work_root: Path | None,
) -> dict[str, Any]:
    case = manifest.case(case_id)
    source = None
    source_reason = None
    if source_root is not None:
        candidate = (source_root / case.project_id).resolve()
        if candidate.is_dir():
            source = candidate
        else:
            source_reason = f"source checkout unavailable: {candidate}"
    runs: list[dict[str, Any]] = []
    for index in range(repeats):
        label = "cold" if index == 0 else "warm"
        artifact_path = output_dir / f"{case.case_id}-{label}-{index + 1}.json"
        artifact = run_case(
            manifest.path,
            case.case_id,
            artifact_path,
            source_override=source,
            scanner=scanner,
            docker=docker,
            work_root=work_root,
        )
        normalized = _analyze_record(artifact)
        runs.append(
            {
                "index": index + 1,
                "phase": label,
                "artifact": f"{output_dir.name}/{artifact_path.name}",
                "status": artifact.get("status"),
                "oracle_status": next(
                    (
                        phase.get("status")
                        for phase in artifact.get("phases", [])
                        if phase.get("name") == "oracle"
                    ),
                    "unavailable",
                ),
                "normalized": normalized,
            }
        )
    comparison = _case_comparison(runs)
    status = _case_status(comparison, runs)
    result: dict[str, Any] = {
        "case_id": case.case_id,
        "project_id": case.project_id,
        "analysis_mode": case.analysis_mode,
        "profile": case.profile,
        "source": str(source) if source else None,
        "source_reason": source_reason,
        "runs": runs,
        "comparison": comparison,
        "status": status,
    }
    return result


def run_pilot(
    manifest_path: Path,
    output: Path,
    *,
    source_root: Path | None = None,
    scanner: tuple[str, ...] | None = None,
    repeats: int = 3,
    docker: DockerAdapter | None = None,
    work_root: Path | None = None,
) -> dict[str, Any]:
    if repeats < 1 or repeats > 3:
        raise SandboxManifestError("pilot repeats must be between 1 and 3")
    manifest = load_manifest(manifest_path)
    output_dir = output.parent / f"{output.stem}-runs"
    output_dir.mkdir(parents=True, exist_ok=True)
    resolved_source_root = source_root.resolve() if source_root else None
    cases = [
        _case_summary(
            manifest,
            case.case_id,
            output_dir,
            resolved_source_root,
            scanner,
            repeats,
            docker,
            work_root,
        )
        for case in manifest.cases
    ]
    statuses = {case["status"] for case in cases}
    if "drift" in statuses:
        status = "drift"
    elif "unavailable" in statuses:
        status = "unavailable"
    elif statuses == {"passed"}:
        status = "passed"
    else:
        status = "failed"
    summary = {
        "schema_version": PILOT_SCHEMA_VERSION,
        "kind": "pilot-summary",
        "protocol": PROTOCOL,
        "corpus": manifest.corpus,
        "run_id": uuid.uuid4().hex,
        "manifest_sha256": manifest.sha256,
        "repeats": repeats,
        "repeat_policy": ["cold", "warm", "warm"][:repeats],
        "source_root": str(resolved_source_root) if resolved_source_root else None,
        "scanner": _redact_command(scanner) if scanner else None,
        "cases": cases,
        "status": status,
        "reason": (
            "at least one case has normalized-output drift"
            if status == "drift"
            else "at least one case is unavailable"
            if status == "unavailable"
            else "all cases have stable normalized output"
        ),
    }
    _write_summary(output, summary)
    return summary
