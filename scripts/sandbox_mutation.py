"""Controlled mutation packet orchestration for sandbox cases."""

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


MUTATION_SCHEMA_VERSION = 1


def _write_summary(path: Path, summary: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{uuid.uuid4().hex}.tmp")
    temporary.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    temporary.replace(path)


def _phase(artifact: dict[str, Any], name: str) -> dict[str, Any]:
    return next(
        (phase for phase in artifact.get("phases", []) if phase.get("name") == name),
        {"name": name, "status": "unavailable", "reason": "phase missing"},
    )


def _analysis(artifact: dict[str, Any]) -> dict[str, Any]:
    phase = _phase(artifact, "analyze")
    result = phase.get("result", {})
    normalized = (
        result.get("normalized_findings", {}) if isinstance(result, dict) else {}
    )
    if not isinstance(normalized, dict):
        normalized = {}
    return {
        "status": normalized.get("status", "unavailable"),
        "count": normalized.get("count"),
        "sha256": normalized.get("sha256"),
        "reason": normalized.get("reason") or phase.get("reason"),
    }


def _case_status(artifact: dict[str, Any], expected_oracle: str) -> str:
    phases = {
        name: _phase(artifact, name)
        for name in ("baseline", "mutate", "oracle", "revert")
    }
    setup_phases = (phases["baseline"], phases["mutate"], phases["revert"])
    if any(phase["status"] in {"unavailable", "timeout"} for phase in setup_phases):
        return "unavailable"
    if any(phase["status"] != "passed" for phase in setup_phases):
        return "failed"
    oracle_status = phases["oracle"]["status"]
    if oracle_status in {"unavailable", "timeout"}:
        return "unavailable"
    return "passed" if oracle_status == expected_oracle else "failed"


def _case_summary(
    manifest: SandboxManifest,
    case_id: str,
    output_dir: Path,
    source_root: Path | None,
    scanner: tuple[str, ...] | None,
    docker: DockerAdapter | None,
    work_root: Path | None,
) -> dict[str, Any]:
    case = manifest.case(case_id)
    if case.mutation_kind not in {"violation", "negative-control"}:
        raise SandboxManifestError(
            f"case {case.case_id}: mutation command requires a mutation kind"
        )
    source = None
    source_reason = None
    if source_root is not None:
        candidate = (source_root / case.project_id).resolve()
        if candidate.is_dir():
            source = candidate
        else:
            source_reason = f"source checkout unavailable: {candidate}"
    artifact_path = output_dir / f"{case.case_id}.json"
    artifact = run_case(
        manifest.path,
        case.case_id,
        artifact_path,
        source_override=source,
        scanner=scanner,
        docker=docker,
        work_root=work_root,
    )
    phases = {
        name: _phase(artifact, name)
        for name in ("baseline", "mutate", "oracle", "revert")
    }
    return {
        "case_id": case.case_id,
        "project_id": case.project_id,
        "mutation_kind": case.mutation_kind,
        "split": case.split,
        "expected_oracle": case.expected_oracle,
        "source": str(source) if source else None,
        "source_reason": source_reason,
        "artifact": f"{output_dir.name}/{artifact_path.name}",
        "status": _case_status(artifact, case.expected_oracle),
        "artifact_status": artifact.get("status"),
        "analysis": _analysis(artifact),
        "phases": {
            name: {"status": phase.get("status"), "reason": phase.get("reason")}
            for name, phase in phases.items()
        },
        "oracle_status": phases["oracle"].get("status"),
    }


def run_mutation_packet(
    manifest_path: Path,
    output: Path,
    *,
    source_root: Path | None = None,
    scanner: tuple[str, ...] | None = None,
    docker: DockerAdapter | None = None,
    work_root: Path | None = None,
) -> dict[str, Any]:
    manifest = load_manifest(manifest_path)
    mutation_cases = [
        case
        for case in manifest.cases
        if case.mutation_kind in {"violation", "negative-control"}
    ]
    if not mutation_cases:
        raise SandboxManifestError("manifest contains no mutation cases")
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
            docker,
            work_root,
        )
        for case in mutation_cases
    ]
    statuses = {case["status"] for case in cases}
    if "failed" in statuses:
        status = "failed"
    elif "unavailable" in statuses:
        status = "unavailable"
    else:
        status = "passed"
    summary = {
        "schema_version": MUTATION_SCHEMA_VERSION,
        "kind": "mutation-summary",
        "protocol": PROTOCOL,
        "corpus": manifest.corpus,
        "run_id": uuid.uuid4().hex,
        "manifest_sha256": manifest.sha256,
        "source_root": str(resolved_source_root) if resolved_source_root else None,
        "scanner": _redact_command(scanner) if scanner else None,
        "cases": cases,
        "status": status,
        "reason": (
            "at least one mutation case failed its independent oracle"
            if status == "failed"
            else "at least one mutation case is unavailable"
            if status == "unavailable"
            else "all mutation cases matched their expected oracle states"
        ),
    }
    _write_summary(output, summary)
    return summary
