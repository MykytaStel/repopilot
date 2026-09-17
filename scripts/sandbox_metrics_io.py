"""Persist and validate sandbox metrics artifacts."""

from __future__ import annotations

import json
import uuid
from pathlib import Path
from typing import Any

from sandbox_contract import SandboxManifestError
from sandbox_metrics import build_metrics


def load_metrics(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise SandboxManifestError(
            f"cannot read sandbox metrics {path}: {error}"
        ) from error
    if not isinstance(data, dict):
        raise SandboxManifestError("sandbox metrics must be a JSON object")
    return data


def write_metrics(
    output_path: Path, summary_path: Path, manifest_path: Path
) -> dict[str, Any]:
    result = build_metrics(summary_path, manifest_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    temporary = output_path.with_name(f".{output_path.name}.{uuid.uuid4().hex}.tmp")
    temporary.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    temporary.replace(output_path)
    return result


def validate_metrics(
    metrics_path: Path, summary_path: Path, manifest_path: Path
) -> dict[str, Any]:
    actual = load_metrics(metrics_path)
    expected = build_metrics(summary_path, manifest_path)
    if actual != expected:
        raise SandboxManifestError(
            "sandbox metrics artifact does not match recomputed metrics"
        )
    return {
        "status": "valid",
        "cases": len(expected["cases"]),
        "source_summary_kind": expected["source_summary_kind"],
    }
