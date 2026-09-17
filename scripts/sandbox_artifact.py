"""Validation for canonical sandbox result artifacts."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from sandbox_contract import (
    ALLOWED_PHASES,
    ALLOWED_STATUSES,
    PROTOCOL,
    SCHEMA_VERSION,
    SHA1,
    SHA256,
    SandboxManifestError,
)
from sandbox_manifest import load_manifest


def validate_artifact(path: Path, manifest_path: Path) -> dict[str, Any]:
    manifest = load_manifest(manifest_path)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise SandboxManifestError(
            f"cannot read sandbox artifact {path}: {error}"
        ) from error
    if not isinstance(data, dict):
        raise SandboxManifestError("sandbox artifact must be a JSON object")
    if data.get("schema_version") != SCHEMA_VERSION or data.get("protocol") != PROTOCOL:
        raise SandboxManifestError("sandbox artifact schema/protocol mismatch")
    if data.get("manifest_sha256") != manifest.sha256:
        raise SandboxManifestError(
            "sandbox artifact manifest_sha256 does not match manifest"
        )
    case = manifest.case(str(data.get("case_id")))
    if data.get("project_id") != case.project_id:
        raise SandboxManifestError("sandbox artifact project does not match case")
    if data.get("status") not in ALLOWED_STATUSES:
        raise SandboxManifestError("sandbox artifact has unsupported status")
    phases = data.get("phases")
    if not isinstance(phases, list) or any(
        not isinstance(phase, dict)
        or phase.get("name") not in ALLOWED_PHASES
        or phase.get("status") not in ALLOWED_STATUSES
        for phase in phases
    ):
        raise SandboxManifestError("sandbox artifact phases are invalid")
    cleanup = data.get("cleanup")
    if not isinstance(cleanup, dict) or cleanup.get("status") not in {
        "complete",
        "partial",
        "failed",
    }:
        raise SandboxManifestError("sandbox artifact requires a cleanup receipt")
    inputs = data.get("inputs")
    if not isinstance(inputs, dict):
        raise SandboxManifestError("sandbox artifact requires input provenance")
    project = manifest.project(case.project_id)
    if inputs.get("project_sha") != project.sha or inputs.get("image") != case.image:
        raise SandboxManifestError(
            "sandbox artifact project SHA or image does not match manifest"
        )
    source_sha = inputs.get("source_sha")
    if source_sha is not None and (
        not isinstance(source_sha, str) or not SHA1.fullmatch(source_sha)
    ):
        raise SandboxManifestError("sandbox artifact source_sha is invalid")
    patch_sha = inputs.get("patch_sha256")
    if patch_sha is not None and (
        not isinstance(patch_sha, str) or not SHA256.fullmatch(patch_sha)
    ):
        raise SandboxManifestError("sandbox artifact patch_sha256 is invalid")
    return {
        "status": "valid",
        "case_id": case.case_id,
        "project_id": case.project_id,
        "phases": len(phases),
    }
