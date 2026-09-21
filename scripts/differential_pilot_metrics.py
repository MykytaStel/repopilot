"""Score a single-expert differential pilot as exploratory evidence."""

from __future__ import annotations

import hashlib
import json
import math
import statistics
from pathlib import Path
from typing import Any

from differential_novelty import evidence_rule_ids
from differential_pilot import load_pilot
from differential_telemetry import event_elapsed_ms
from real_history_contract import HoldoutManifestError


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def wilson_interval(successes: int, trials: int, z: float = 1.96) -> tuple[float, float] | None:
    if trials < 0 or successes < 0 or successes > trials:
        raise HoldoutManifestError("invalid pilot metric counts")
    if trials == 0:
        return None
    proportion = successes / trials
    denominator = 1 + z * z / trials
    center = (proportion + z * z / (2 * trials)) / denominator
    margin = z * math.sqrt(proportion * (1 - proportion) / trials + z * z / (4 * trials * trials)) / denominator
    return max(0.0, center - margin), min(1.0, center + margin)


def _metric(successes: int, trials: int) -> dict[str, object]:
    interval = wilson_interval(successes, trials)
    return {
        "successes": successes,
        "trials": trials,
        "value": successes / trials if trials else None,
        "wilson_95": list(interval) if interval else None,
    }


def _load_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HoldoutManifestError(f"cannot read differential artifact {path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError("differential artifact must be an object")
    return data


def _median(values: list[float]) -> float | None:
    return round(statistics.median(values), 3) if values else None


def _resource_phase(run: dict[str, Any]) -> str | None:
    phase = run.get("resource_phase")
    if phase in {"cold", "warm"}:
        return phase
    repeat = run.get("repeat")
    if repeat == 1:
        return "cold"
    if isinstance(repeat, int) and repeat > 1:
        return "warm"
    return None


def _case_measurements(observation: dict[str, Any]) -> dict[str, object]:
    baseline_values = [
        float(run["wall_ms"])
        for runs in observation["baselines"].values()
        for run in runs
        if isinstance(run, dict) and isinstance(run.get("wall_ms"), (int, float))
    ]
    reviews = [run for run in observation["reviews"] if isinstance(run, dict)]
    review_values = [float(run["wall_ms"]) for run in reviews if isinstance(run.get("wall_ms"), (int, float))]
    rss_values = [
        float(run["child_max_rss_kb"])
        for run in reviews
        if isinstance(run.get("child_max_rss_kb"), (int, float))
    ]
    rss_by_phase: dict[str, list[float]] = {"cold": [], "warm": []}
    for review in reviews:
        sample = review.get("child_max_rss_kb")
        phase = _resource_phase(review)
        if isinstance(sample, (int, float)) and phase is not None:
            rss_by_phase[phase].append(float(sample))
    first_evidence_values = []
    decision_values = []
    for review in reviews:
        telemetry = review.get("telemetry")
        evidence_ms = event_elapsed_ms(telemetry, "evidence_ready") if isinstance(telemetry, dict) else None
        decision_ms = event_elapsed_ms(telemetry, "decision_ready") if isinstance(telemetry, dict) else None
        if evidence_ms is not None:
            first_evidence_values.append(evidence_ms)
        if decision_ms is not None:
            decision_values.append(decision_ms)
    duplicate = _duplicate_work_measurement(observation)
    return {
        "median_baseline_wall_ms": _median(baseline_values),
        "median_review_wall_ms": _median(review_values),
        "median_review_child_max_rss_kb": _median(rss_values),
        "median_review_child_max_rss_kb_by_phase": {
            phase: _median(values) for phase, values in rss_by_phase.items()
        },
        "time_to_first_useful_evidence": _timing_measurement(
            first_evidence_values, "no useful-evidence event was recorded"
        ),
        "decision_latency": _timing_measurement(decision_values, "no decision event was recorded"),
        "duplicate_work": duplicate,
    }


def _timing_measurement(values: list[float], reason: str) -> dict[str, object]:
    if not values:
        return {"status": "unavailable", "reason": reason}
    return {"status": "measured", "median_ms": _median(values), "samples": len(values)}


def _duplicate_work_measurement(observation: dict[str, Any]) -> dict[str, object]:
    baseline_keys: set[str] = set()
    needs_verification = False
    used_static_mapping = False
    for runs in observation["baselines"].values():
        for run in runs:
            evidence = run.get("evidence")
            if not isinstance(evidence, dict) or evidence.get("status") != "measured":
                return {
                    "status": "unavailable",
                    "reason": "baseline command has no normalized evidence adapter",
                }
            comparison = evidence.get("comparison")
            if not isinstance(comparison, dict) or comparison.get("status") != "measured":
                if (
                    evidence.get("status") == "measured"
                    and comparison
                    and comparison.get("scheme") == "review-exact-v1"
                ):
                    needs_verification = True
                    keys = evidence.get("keys")
                    if not isinstance(keys, list) or not all(isinstance(key, str) for key in keys):
                        return {
                            "status": "unavailable",
                            "reason": "baseline evidence has invalid normalized identity keys",
                        }
                    baseline_keys.update(keys)
                    continue
                return {
                    "status": "unavailable",
                    "reason": "baseline evidence has no review-comparable identity mapping",
                }
            if comparison.get("scheme") != "review-exact-v1":
                return {"status": "unavailable", "reason": "baseline comparison identity scheme is unsupported"}
            keys = comparison.get("keys")
            if not isinstance(keys, list) or not all(isinstance(key, str) for key in keys):
                return {"status": "unavailable", "reason": "baseline comparison keys are invalid"}
            baseline_keys.update(keys)
            used_static_mapping = True
    review_keys: set[str] = set()
    verification_keys: set[str] = set()
    for review in observation["reviews"]:
        keys = review.get("in_diff_comparison_keys")
        if not isinstance(keys, list):
            keys = review.get("in_diff_evidence_keys", [])
        review_keys.update(key for key in keys if isinstance(key, str))
        verification = review.get("verification_comparison")
        if needs_verification:
            if not isinstance(verification, dict) or verification.get("status") != "measured":
                return {
                    "status": "unavailable",
                    "reason": "baseline evidence has no review-comparable identity mapping",
                }
            if verification.get("scheme") != "review-verification-v1":
                return {"status": "unavailable", "reason": "review verification identity scheme is unsupported"}
            keys = verification.get("keys")
            if not isinstance(keys, list) or not all(isinstance(key, str) for key in keys):
                return {"status": "unavailable", "reason": "review verification keys are invalid"}
            verification_keys.update(keys)
    if needs_verification:
        review_keys.update(verification_keys)
        identity_source = "mixed" if used_static_mapping else "review-verification-v1"
    else:
        identity_source = "review-exact-v1"
    overlap_count = len(baseline_keys & review_keys)
    return {
        "status": "measured",
        "identity_source": identity_source,
        "overlap_count": overlap_count,
        "baseline_evidence_count": len(baseline_keys),
        "review_evidence_count": len(review_keys),
        "overlap_rate": round(overlap_count / len(review_keys), 3) if review_keys else None,
    }


def build_pilot_metrics(
    artifact_path: Path,
    pilot_path: Path,
    manifest_path: Path,
    differential_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    pilot_data, pilot_cases = load_pilot(
        pilot_path, artifact_path, manifest_path, differential_path, rules_reference, zoo_manifest
    )
    artifact = _load_json(artifact_path)
    observations = {case["id"]: case for case in artifact["cases"]}
    scores = _collect_pilot_scores(pilot_cases, observations)
    counts = scores["counts"]
    cases = scores["cases"]
    actual_positives = counts["tp"] + counts["fn"]
    actual_negatives = counts["tn"] + counts["fp"]
    predicted_positives = counts["tp"] + counts["fp"]
    return {
        "schema_version": 1,
        "corpus": artifact["corpus"],
        "protocol": "single-expert-pilot-v1",
        "scope": "single-expert exploratory pilot",
        "limitation": (
            "model-assisted or single-reviewer evidence; not independent validation "
            "or a production/language-wide estimate"
        ),
        "manifest_sha256": sha256_file(manifest_path),
        "differential_artifact_sha256": sha256_file(artifact_path),
        "pilot_sha256": sha256_file(pilot_path),
        "reviewer": pilot_data["reviewer"],
        "cases": cases,
        "counts": counts,
        "metrics": {
            "recall": _metric(counts["tp"], actual_positives),
            "specificity": _metric(counts["tn"], actual_negatives),
            "precision": _metric(counts["tp"], predicted_positives),
            "case_coverage": _metric(actual_positives + actual_negatives, len(cases)),
        },
        "measurements": scores["measurements"],
    }


def _collect_pilot_scores(
    pilot_cases: dict[str, dict[str, Any]], observations: dict[str, dict[str, Any]]
) -> dict[str, object]:
    counts = {key: 0 for key in ("tp", "fn", "tn", "fp", "excluded")}
    cases: list[dict[str, object]] = []
    deterministic_cases = 0
    novel_evidence_cases = 0
    baseline_times: list[float] = []
    review_times: list[float] = []
    rss_times: list[float] = []
    rss_times_by_phase: dict[str, list[float]] = {"cold": [], "warm": []}
    first_evidence_times: list[float] = []
    decision_latencies: list[float] = []
    duplicate_measurements: list[dict[str, object]] = []
    for case_id, label in pilot_cases.items():
        observation = observations.get(case_id)
        if observation is None:
            raise HoldoutManifestError(f"differential artifact is missing case {case_id}")
        scored = _score_pilot_case(case_id, label, observation)
        counts[scored["outcome"]] += 1
        if scored["novel"]:
            novel_evidence_cases += 1
        if scored["deterministic"]:
            deterministic_cases += 1
        case_measurements = scored["measurements"]
        for values, key in (
            (first_evidence_times, "time_to_first_useful_evidence"),
            (decision_latencies, "decision_latency"),
        ):
            measurement = case_measurements[key]
            if measurement.get("status") == "measured" and isinstance(measurement.get("median_ms"), (int, float)):
                values.append(float(measurement["median_ms"]))
        duplicate_measurements.append(case_measurements["duplicate_work"])
        for values, key in (
            (baseline_times, "median_baseline_wall_ms"),
            (review_times, "median_review_wall_ms"),
            (rss_times, "median_review_child_max_rss_kb"),
        ):
            value = case_measurements[key]
            if isinstance(value, (int, float)):
                values.append(float(value))
        for phase, values in rss_times_by_phase.items():
            value = case_measurements["median_review_child_max_rss_kb_by_phase"][phase]
            if isinstance(value, (int, float)):
                values.append(float(value))
        cases.append(scored["case"])
    return {
        "counts": counts,
        "cases": cases,
        "measurements": _build_measurement_summary(
            cases,
            deterministic_cases,
            novel_evidence_cases,
            baseline_times,
            review_times,
            rss_times,
            rss_times_by_phase,
            first_evidence_times,
            decision_latencies,
            duplicate_measurements,
        ),
    }


def _build_measurement_summary(
    cases: list[dict[str, object]],
    deterministic_cases: int,
    novel_evidence_cases: int,
    baseline_times: list[float],
    review_times: list[float],
    rss_times: list[float],
    rss_times_by_phase: dict[str, list[float]],
    first_evidence_times: list[float],
    decision_latencies: list[float],
    duplicate_measurements: list[dict[str, object]],
) -> dict[str, object]:
    return {
        "deterministic_cases": deterministic_cases,
        "novel_evidence_cases": novel_evidence_cases,
        "case_count": len(cases),
        "median_baseline_wall_ms": _median(baseline_times),
        "median_review_wall_ms": _median(review_times),
        "median_review_child_max_rss_kb": _median(rss_times),
        "median_review_child_max_rss_kb_by_phase": {
            phase: _median(values) for phase, values in rss_times_by_phase.items()
        },
        "time_to_first_useful_evidence": _timing_measurement(
            first_evidence_times, "no useful-evidence events were recorded"
        ),
        "decision_latency": _timing_measurement(
            decision_latencies, "no decision events were recorded"
        ),
        "duplicate_work": _aggregate_duplicate_work(duplicate_measurements),
    }


def _score_pilot_case(
    case_id: str, label: dict[str, Any], observation: dict[str, Any]
) -> dict[str, object]:
    novel_keys = sorted(
        {
            key
            for review in observation["reviews"]
            if isinstance(review, dict)
            for key in review.get("novel_in_diff_evidence_keys", [])
        }
    )
    observed_rules = sorted(evidence_rule_ids(novel_keys))
    expected_rules = sorted(set(label["expected_rule_ids"]))
    if label["label"] == "defect-present":
        outcome = "tp" if set(expected_rules) & set(observed_rules) else "fn"
    elif label["label"] == "no-defect":
        outcome = "fp" if observed_rules else "tn"
    else:
        outcome = "excluded"
    measurements = _case_measurements(observation)
    return {
        "outcome": outcome,
        "novel": bool(novel_keys),
        "deterministic": observation["determinism"].get("stable_evidence_deterministic") is True,
        "measurements": measurements,
        "case": {
            "id": case_id,
            "label": label["label"],
            "expected_rule_ids": expected_rules,
            "observed_novel_rule_ids": observed_rules,
            "novel_evidence_count": len(novel_keys),
            "outcome": outcome,
            "measurements": measurements,
        },
    }


def _aggregate_duplicate_work(measurements: list[dict[str, object]]) -> dict[str, object]:
    unavailable = [measurement for measurement in measurements if measurement.get("status") != "measured"]
    if not measurements or unavailable:
        reasons = sorted(
            {
                str(measurement.get("reason"))
                for measurement in unavailable
                if isinstance(measurement.get("reason"), str) and measurement["reason"]
            }
        )
        return {
            "status": "unavailable",
            "reason": reasons[0] if len(reasons) == 1 else "baseline overlap classification is not captured",
            "cases_measured": len(measurements) - len(unavailable),
            "cases_unavailable": len(unavailable),
        }
    overlap_count = sum(int(measurement.get("overlap_count", 0)) for measurement in measurements)
    baseline_count = sum(int(measurement.get("baseline_evidence_count", 0)) for measurement in measurements)
    review_count = sum(int(measurement.get("review_evidence_count", 0)) for measurement in measurements)
    identity_sources = sorted(
        {
            str(measurement["identity_source"])
            for measurement in measurements
            if isinstance(measurement.get("identity_source"), str)
        }
    )
    return {
        "status": "measured",
        "identity_source": identity_sources[0] if len(identity_sources) == 1 else "mixed",
        "overlap_count": overlap_count,
        "baseline_evidence_count": baseline_count,
        "review_evidence_count": review_count,
        "overlap_rate": round(overlap_count / review_count, 3) if review_count else None,
        "cases": len(measurements),
    }


def write_pilot_metrics(
    output_path: Path,
    artifact_path: Path,
    pilot_path: Path,
    manifest_path: Path,
    differential_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    result = build_pilot_metrics(
        artifact_path, pilot_path, manifest_path, differential_path, rules_reference, zoo_manifest
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def validate_pilot_metrics(
    metrics_path: Path,
    artifact_path: Path,
    pilot_path: Path,
    manifest_path: Path,
    differential_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    try:
        actual = json.loads(metrics_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HoldoutManifestError(f"cannot read differential pilot metrics {metrics_path}: {error}") from error
    if not isinstance(actual, dict):
        raise HoldoutManifestError("differential pilot metrics must be a JSON object")
    expected = build_pilot_metrics(
        artifact_path,
        pilot_path,
        manifest_path,
        differential_path,
        rules_reference,
        zoo_manifest,
    )
    if actual != expected:
        raise HoldoutManifestError("differential pilot metrics artifact does not match recomputed score")
    return {
        "status": "valid",
        "protocol": expected["protocol"],
        "scope": expected["scope"],
        "reviewer": expected["reviewer"],
        "cases": len(expected["cases"]),
        "counts": expected["counts"],
        "measurements": expected["measurements"],
    }
