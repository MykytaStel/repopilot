"""Cross-manifest validation for the differential utility protocol."""

from __future__ import annotations

from pathlib import Path

from differential_case_validation import validate_case_alignment
from differential_manifest import DifferentialManifestError, load_differential
from real_history_contract import validate_manifest


SCHEMA_VERSION = 1


def validate_differential(
    path: Path,
    holdout_manifest: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    """Return a deterministic protocol summary when both manifests agree."""
    corpus, protocol, repetitions, measurements, cases = load_differential(path)
    holdout_corpus, holdout_protocol, holdout_cases = validate_manifest(
        holdout_manifest, rules_reference, zoo_manifest
    )
    if corpus != holdout_corpus:
        raise DifferentialManifestError("differential corpus does not match holdout corpus")
    if any(case.source_kind != "merged-pull-request" for case in holdout_cases):
        raise DifferentialManifestError("differential cases require merged pull-request holdout sources")
    validate_case_alignment(holdout_cases, cases)
    return {
        "schema_version": SCHEMA_VERSION,
        "corpus": corpus,
        "protocol": protocol,
        "holdout_protocol": holdout_protocol,
        "repetitions": repetitions,
        "measurements": list(measurements),
        "cases": len(cases),
        "baseline_ids": sorted({baseline for case in cases for baseline in case.baseline_ids}),
        "status": "valid",
    }
