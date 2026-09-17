"""Read, validate and render sandbox summary files."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from sandbox_contract import (
    SandboxManifestError,
    validate_mutation_summary,
    validate_pilot_summary,
)
from sandbox_report import render_report


def _load_summary(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise SandboxManifestError(
            f"cannot read sandbox summary {path}: {error}"
        ) from error
    if not isinstance(data, dict):
        raise SandboxManifestError("sandbox summary must be a JSON object")
    return data


def report_summary(
    path: Path, manifest_path: Path, output_format: str = "markdown"
) -> str:
    """Validate a summary against its manifest, then render it."""
    summary = _load_summary(path)
    if summary.get("kind") == "pilot-summary":
        validate_pilot_summary(path, manifest_path)
    elif summary.get("kind") == "mutation-summary":
        validate_mutation_summary(path, manifest_path)
    else:
        raise SandboxManifestError(
            "sandbox report requires pilot-summary or mutation-summary"
        )
    return render_report(summary, output_format)
