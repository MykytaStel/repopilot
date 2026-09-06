"""Parsing primitives for the preregistered differential utility manifest."""

from __future__ import annotations

import tomllib
from dataclasses import dataclass
from pathlib import Path

from real_history_contract import BASELINES, HoldoutManifestError


SCHEMA_VERSION = 1
PROTOCOL = "differential-utility-v1"
MEASUREMENTS = {
    "novel-actionable-evidence",
    "duplicate-work",
    "time-to-first-useful-evidence",
    "decision-latency",
    "determinism",
    "resource-cost",
}


@dataclass(frozen=True)
class DifferentialCase:
    case_id: str
    baseline_ids: tuple[str, ...]


class DifferentialManifestError(HoldoutManifestError):
    """Raised when the preregistered utility benchmark is unsafe or incomplete."""


def _parse_case(raw: object, index: int) -> DifferentialCase:
    if not isinstance(raw, dict):
        raise DifferentialManifestError(f"case {index}: entry must be a table")
    case_id, baseline_ids = raw.get("id"), raw.get("baseline_ids")
    if not isinstance(case_id, str) or not case_id.strip():
        raise DifferentialManifestError(f"case {index}: id must be a non-empty string")
    if not isinstance(baseline_ids, list) or not baseline_ids or not all(
        isinstance(item, str) for item in baseline_ids
    ):
        raise DifferentialManifestError(f"case {case_id}: baseline_ids must be a non-empty string array")
    unknown = set(baseline_ids) - BASELINES
    if unknown:
        raise DifferentialManifestError(f"case {case_id}: unknown baselines: {', '.join(sorted(unknown))}")
    return DifferentialCase(case_id.strip(), tuple(baseline_ids))


def load_differential(path: Path) -> tuple[str, str, int, tuple[str, ...], list[DifferentialCase]]:
    try:
        document = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise DifferentialManifestError(f"cannot read differential manifest {path}: {error}") from error
    if document.get("schema_version") != SCHEMA_VERSION:
        raise DifferentialManifestError(f"schema_version must be {SCHEMA_VERSION}")
    corpus, protocol = document.get("corpus"), document.get("protocol")
    if not isinstance(corpus, str) or not corpus.strip():
        raise DifferentialManifestError("corpus must be a non-empty string")
    if protocol != PROTOCOL:
        raise DifferentialManifestError(f"protocol must be {PROTOCOL}")
    repetitions = document.get("repetitions")
    if not isinstance(repetitions, int) or repetitions < 2:
        raise DifferentialManifestError("repetitions must be an integer of at least 2")
    measurements = document.get("measurements")
    if not isinstance(measurements, list) or set(measurements) != MEASUREMENTS:
        raise DifferentialManifestError("measurements must enumerate the fixed utility metric set")
    raw_cases = document.get("case")
    if not isinstance(raw_cases, list) or not raw_cases:
        raise DifferentialManifestError("differential manifest must contain cases")
    cases = [_parse_case(raw, index) for index, raw in enumerate(raw_cases, start=1)]
    ids = [case.case_id for case in cases]
    if len(set(ids)) != len(ids):
        raise DifferentialManifestError("case IDs must be unique")
    return corpus.strip(), protocol, repetitions, tuple(sorted(measurements)), cases
