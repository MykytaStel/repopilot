"""Controlled fixture signal-kind/tier measurements (not production accuracy)."""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path

from changeproof_benchmark_contract import BenchmarkCase, BenchmarkManifestError, fixture_directory


BUCKETS = ("definitely", "maybe", "noise")
KIND = re.compile(r"^[a-z0-9][a-z0-9._-]*$")


def _identity(bucket: object, kind: object) -> str:
    if bucket not in BUCKETS or not isinstance(kind, str) or not KIND.fullmatch(kind):
        raise BenchmarkManifestError("invalid signal kind or tier")
    return f"{bucket}/{kind}"


def normalize_signal_kinds(report: dict) -> list[str]:
    tiers = report.get("tiered_signals")
    if not isinstance(tiers, dict):
        raise BenchmarkManifestError("review has no tiered_signals object")
    identities = set()
    for bucket in BUCKETS:
        signals = tiers.get(bucket)
        if not isinstance(signals, list):
            raise BenchmarkManifestError(f"review signal tier {bucket} is unavailable")
        for signal in signals:
            if not isinstance(signal, dict):
                raise BenchmarkManifestError("review signal must be an object")
            identities.add(_identity(bucket, signal.get("kind")))
    return sorted(identities)


def validate_signal_kinds(value: object) -> list[str]:
    if not isinstance(value, list) or any(not isinstance(item, str) for item in value):
        raise BenchmarkManifestError("signal_kinds must be a string array")
    for item in value:
        bucket, separator, kind = item.partition("/")
        if not separator:
            raise BenchmarkManifestError("signal_kinds entries require a tier")
        _identity(bucket, kind)
    if value != sorted(set(value)):
        raise BenchmarkManifestError("signal_kinds must be sorted and unique")
    return value


def signal_oracle(case: BenchmarkCase, root: Path) -> dict:
    path = fixture_directory(case, root) / "expected.json"
    raw = path.read_bytes()
    try:
        data = json.loads(raw)
    except (ValueError, UnicodeError) as error:
        raise BenchmarkManifestError(f"case {case.case_id}: invalid signal oracle") from error
    if not isinstance(data, dict) or not isinstance(data.get("expect", []), list):
        raise BenchmarkManifestError(f"case {case.case_id}: invalid signal oracle")
    expected = set()
    for item in data.get("expect", []):
        if not isinstance(item, dict):
            raise BenchmarkManifestError(f"case {case.case_id}: invalid signal oracle entry")
        expected.add(_identity(item.get("bucket"), item.get("kind")))
    if (case.variant == "safe" and expected) or (case.variant == "unsafe" and not expected):
        raise BenchmarkManifestError(f"case {case.case_id}: missing or inconsistent signal oracle")
    return {"sha256": hashlib.sha256(raw).hexdigest(), "expected": sorted(expected)}


def summarize_signals(cases: list[dict]) -> dict:
    result = {
        "measured_cases": 0,
        "unavailable_cases": 0,
        "nondeterministic_cases": 0,
        "positive_expected": 0,
        "positive_detected": 0,
        "safe_cases": 0,
        "safe_cases_with_signals": 0,
        "by_signal": {},
        "misses": [],
        "safe_alerts": [],
    }
    for entry in cases:
        case, runs = entry["case"], entry["runs"]
        oracle = entry.get("signal_oracle")
        if oracle is None:
            result["unavailable_cases"] += 1
            continue
        observations = [run["signal_kinds"] for run in runs]
        if not entry["deterministic"] or any(value != observations[0] for value in observations):
            result["nondeterministic_cases"] += 1
            continue
        result["measured_cases"] += 1
        observed = set(observations[0])
        expected = set(oracle["expected"])
        if case.variant == "safe":
            result["safe_cases"] += 1
            if observed:
                result["safe_cases_with_signals"] += 1
                result["safe_alerts"].append({"id": case.case_id, "signals": sorted(observed)})
        else:
            result["positive_expected"] += len(expected)
            result["positive_detected"] += len(expected & observed)
            if expected - observed:
                result["misses"].append({"id": case.case_id, "signals": sorted(expected - observed)})
        for identity in sorted(expected | (observed if case.variant == "safe" else set())):
            counts = result["by_signal"].setdefault(identity, {
                "expected": 0, "detected": 0, "missed": 0, "safe_alert_cases": 0,
            })
            if identity in expected:
                counts["expected"] += 1
                counts["detected" if identity in observed else "missed"] += 1
            if case.variant == "safe" and identity in observed:
                counts["safe_alert_cases"] += 1
    result["misses"].sort(key=lambda item: item["id"])
    result["safe_alerts"].sort(key=lambda item: item["id"])
    return result
