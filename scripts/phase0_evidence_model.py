"""Shared model and small provenance helpers for the Phase 0 evidence audit."""

from __future__ import annotations

import hashlib
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

from differential_contract import DifferentialManifestError, validate_differential
from real_history_contract import HoldoutManifestError, validate_manifest


@dataclass(frozen=True)
class Phase0Paths:
    root: Path
    manifest: Path
    differential_manifest: Path
    rules_reference: Path
    zoo_manifest: Path
    real_history_artifact: Path | None = None
    annotation_a: Path | None = None
    annotation_b: Path | None = None
    adjudication: Path | None = None
    real_history_metrics: Path | None = None
    differential_artifact: Path | None = None
    differential_pilot: Path | None = None
    differential_metrics: Path | None = None

    @classmethod
    def defaults(cls, root: Path) -> "Phase0Paths":
        base = root.resolve()
        return cls(
            root=base,
            manifest=base / "tests/benchmarks/manifest.toml",
            differential_manifest=base / "tests/benchmarks/differential.toml",
            rules_reference=base / "docs/rules-reference.md",
            zoo_manifest=base / "tests/zoo/manifest.toml",
        )


def short_error(source: str, error: Exception) -> str:
    message = re.sub(r"\s+", " ", str(error)).strip()
    return f"{source}: {message[:300]}"


def input_hashes(paths: tuple[tuple[str, Path | None], ...]) -> dict[str, str]:
    hashes: dict[str, str] = {}
    for name, path in paths:
        if path is not None and path.is_file():
            hashes[name] = hashlib.sha256(path.read_bytes()).hexdigest()
    return dict(sorted(hashes.items()))


def path_label(path: Path | None, root: Path) -> str | None:
    if path is None:
        return None
    try:
        return str(path.resolve().relative_to(root.resolve()))
    except ValueError:
        return str(path.resolve())


def base_track(track_id: str) -> dict[str, Any]:
    return {
        "id": track_id,
        "protocol_status": "valid",
        "evidence_status": "artifact-missing",
        "label_state": "not-supplied",
        "scope": "protocol-only",
        "hashes": {},
        "limitations": [],
        "next_action": "supply the validated external evidence packet",
    }


def protocol_track(
    track_id: str,
    validator: Callable[[], Any],
    error_name: str,
) -> tuple[dict[str, Any], Any | None]:
    track = base_track(track_id)
    try:
        protocol = validator()
    except (HoldoutManifestError, DifferentialManifestError, OSError) as error:
        track.update(
            protocol_status="invalid",
            evidence_status="protocol-invalid",
            label_state="not-applicable",
            scope="none",
            reason=short_error(error_name, error),
            limitations=["No observation or quality claim is admissible until the protocol is repaired."],
            next_action=f"repair the {track_id} protocol inputs and rerun this report",
        )
        return track, None
    return track, protocol


def validate_real_history_protocol(paths: Phase0Paths) -> tuple[dict[str, Any], Any | None]:
    return protocol_track(
        "real-history",
        lambda: validate_manifest(paths.manifest, paths.rules_reference, paths.zoo_manifest),
        "real-history protocol",
    )


def validate_differential_protocol(paths: Phase0Paths) -> tuple[dict[str, Any], Any | None]:
    return protocol_track(
        "differential-utility",
        lambda: validate_differential(
            paths.differential_manifest,
            paths.manifest,
            paths.rules_reference,
            paths.zoo_manifest,
        ),
        "differential protocol",
    )
