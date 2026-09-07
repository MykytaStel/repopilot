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
    return {
        "median_baseline_wall_ms": _median(baseline_values),
        "median_review_wall_ms": _median(review_values),
        "median_review_child_max_rss_kb": _median(rss_values),
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
    counts = {key: 0 for key in ("tp", "fn", "tn", "fp", "excluded")}
    cases: list[dict[str, object]] = []
    deterministic_cases = 0
    novel_evidence_cases = 0
    baseline_times: list[float] = []
    review_times: list[float] = []
    rss_times: list[float] = []
    for case_id, label in pilot_cases.items():
        observation = observations.get(case_id)
        if observation is None:
            raise HoldoutManifestError(f"differential artifact is missing case {case_id}")
        novel_keys = sorted(
            {
                key
                for review in observation["reviews"]
                if isinstance(review, dict)
                for key in review.get("novel_in_diff_evidence_keys", [])
            }
        )
        observed_rules = sorted(evidence_rule_ids(novel_keys))
        if novel_keys:
            novel_evidence_cases += 1
        expected_rules = sorted(set(label["expected_rule_ids"]))
        if label["label"] == "defect-present":
            outcome = "tp" if set(expected_rules) & set(observed_rules) else "fn"
        elif label["label"] == "no-defect":
            outcome = "fp" if observed_rules else "tn"
        else:
            outcome = "excluded"
        counts[outcome] += 1
        if observation["determinism"].get("stable_evidence_deterministic") is True:
            deterministic_cases += 1
        case_measurements = _case_measurements(observation)
        for values, key in (
            (baseline_times, "median_baseline_wall_ms"),
            (review_times, "median_review_wall_ms"),
            (rss_times, "median_review_child_max_rss_kb"),
        ):
            value = case_measurements[key]
            if isinstance(value, (int, float)):
                values.append(float(value))
        cases.append(
            {
                "id": case_id,
                "label": label["label"],
                "expected_rule_ids": expected_rules,
                "observed_novel_rule_ids": observed_rules,
                "novel_evidence_count": len(novel_keys),
                "outcome": outcome,
                "measurements": case_measurements,
            }
        )
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
        "measurements": {
            "deterministic_cases": deterministic_cases,
            "novel_evidence_cases": novel_evidence_cases,
            "case_count": len(cases),
            "median_baseline_wall_ms": _median(baseline_times),
            "median_review_wall_ms": _median(review_times),
            "median_review_child_max_rss_kb": _median(rss_times),
            "time_to_first_useful_evidence": {"status": "unavailable", "reason": "event timestamps are not captured"},
            "decision_latency": {"status": "unavailable", "reason": "decision events are not captured"},
            "duplicate_work": {"status": "unavailable", "reason": "baseline overlap classification is not captured"},
        },
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
