"""Telemetry primitives for differential utility observations."""

from __future__ import annotations

import math
from typing import Any

TELEMETRY_SCHEMA_VERSION = 1
_EVENT_NAMES = {"process_started", "evidence_ready", "decision_ready", "process_finished"}


def _elapsed_ms(values: Any, keys: tuple[str, ...]) -> float | None:
    if not isinstance(values, dict):
        return None
    micros = [values.get(key, 0) for key in keys]
    if not all(isinstance(value, (int, float)) and value >= 0 for value in micros):
        return None
    return round(sum(float(value) for value in micros) / 1000, 3)


def _event(name: str, elapsed_ms: float) -> dict[str, object]:
    return {"name": name, "elapsed_ms": round(max(0.0, elapsed_ms), 3)}


def build_command_telemetry(wall_ms: float) -> dict[str, object]:
    bounded_wall_ms = round(max(0.0, float(wall_ms)), 3)
    return {
        "schema_version": TELEMETRY_SCHEMA_VERSION,
        "events": [_event("process_started", 0.0), _event("process_finished", bounded_wall_ms)],
    }


def build_review_telemetry(
    report: dict[str, Any], wall_ms: float, useful_evidence: bool
) -> dict[str, object]:
    bounded_wall_ms = round(max(0.0, float(wall_ms)), 3)
    scan_ms = _elapsed_ms(
        report.get("scan_timings"),
        (
            "discovery_us",
            "file_analysis_us",
            "parse_us",
            "file_scan_us",
            "framework_detection_us",
            "post_scan_audits_us",
            "enrichment_us",
            "risk_scoring_us",
            "contract_validation_us",
            "report_finalization_us",
        ),
    )
    review = report.get("review_timings")
    evidence_ms = None
    decision_ms = None
    if scan_ms is not None:
        diff_ms = _elapsed_ms(review, ("diff_loading_us",))
        signal_ms = _elapsed_ms(review, ("review_signals_us",))
        gating_ms = _elapsed_ms(review, ("gating_us",))
        verification_ms = _elapsed_ms(review, ("verification_us",))
        if diff_ms is not None and signal_ms is not None:
            evidence_ms = scan_ms + diff_ms + signal_ms
            if gating_ms is not None and verification_ms is not None:
                decision_ms = evidence_ms + gating_ms + verification_ms
    events = [_event("process_started", 0.0)]
    if useful_evidence and evidence_ms is not None:
        events.append(_event("evidence_ready", min(evidence_ms, bounded_wall_ms)))
    if decision_ms is not None:
        events.append(_event("decision_ready", min(decision_ms, bounded_wall_ms)))
    events.append(_event("process_finished", bounded_wall_ms))
    return {"schema_version": TELEMETRY_SCHEMA_VERSION, "events": events}


def event_elapsed_ms(telemetry: dict[str, Any], name: str) -> float | None:
    for event in telemetry.get("events", []):
        if isinstance(event, dict) and event.get("name") == name:
            value = event.get("elapsed_ms")
            if isinstance(value, (int, float)):
                return float(value)
    return None


def validate_telemetry(telemetry: Any, wall_ms: float, context: str) -> None:
    if not isinstance(telemetry, dict) or telemetry.get("schema_version") != TELEMETRY_SCHEMA_VERSION:
        raise ValueError(f"{context} telemetry schema_version is unsupported")
    events = telemetry.get("events")
    if not isinstance(events, list) or not events:
        raise ValueError(f"{context} telemetry events are missing")
    names: list[str] = []
    elapsed: list[float] = []
    for event in events:
        if not isinstance(event, dict) or not isinstance(event.get("name"), str):
            raise ValueError(f"{context} telemetry event is invalid")
        name = event["name"]
        value = event.get("elapsed_ms")
        if name not in _EVENT_NAMES or name in names:
            raise ValueError(f"{context} telemetry event name is invalid or duplicated")
        if not isinstance(value, (int, float)) or not math.isfinite(float(value)) or value < 0:
            raise ValueError(f"{context} telemetry elapsed_ms is invalid")
        names.append(name)
        elapsed.append(float(value))
    if names[0] != "process_started" or elapsed[0] != 0:
        raise ValueError(f"{context} telemetry must start at process_started")
    if names[-1] != "process_finished" or abs(elapsed[-1] - float(wall_ms)) > 0.001:
        raise ValueError(f"{context} telemetry must finish at wall_ms")
    if any(after < before for before, after in zip(elapsed, elapsed[1:])):
        raise ValueError(f"{context} telemetry events must be monotonic")
    if "evidence_ready" in names and "decision_ready" in names:
        if elapsed[names.index("evidence_ready")] > elapsed[names.index("decision_ready")]:
            raise ValueError(f"{context} telemetry evidence must precede decision")
