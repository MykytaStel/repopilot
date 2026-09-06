"""Compute bounded, corpus-only metrics from an adjudicated holdout."""

from __future__ import annotations

import hashlib
import json
import math
import tomllib
from pathlib import Path
from typing import Any

from real_history_adjudication import validate_adjudication
from real_history_annotations import load_collection
from real_history_contract import HoldoutManifestError


METRICS_SCHEMA_VERSION = 1


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def wilson_interval(successes: int, trials: int, z: float = 1.96) -> tuple[float, float] | None:
    if trials < 0 or successes < 0 or successes > trials:
        raise HoldoutManifestError("invalid binomial counts")
    if trials == 0:
        return None
    proportion = successes / trials
    denominator = 1 + z * z / trials
    center = (proportion + z * z / (2 * trials)) / denominator
    margin = z * math.sqrt(proportion * (1 - proportion) / trials + z * z / (4 * trials * trials)) / denominator
    return (max(0.0, center - margin), min(1.0, center + margin))


def _metric(successes: int, trials: int) -> dict[str, object]:
    interval = wilson_interval(successes, trials)
    return {
        "successes": successes,
        "trials": trials,
        "value": successes / trials if trials else None,
        "wilson_95": list(interval) if interval else None,
    }


def _load_adjudication(path: Path) -> dict[str, Any]:
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise HoldoutManifestError(f"cannot read adjudication {path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError("adjudication must be a TOML table")
    return data


def _case_outcome(gold_label: str, expected_rules: list[str], observed_rules: list[str]) -> str:
    predicted_positive = bool(observed_rules)
    if gold_label == "defect-present":
        return "tp" if any(rule_id in observed_rules for rule_id in expected_rules) else "fn"
    if gold_label == "no-defect":
        return "fp" if predicted_positive else "tn"
    return "excluded"


def build_metrics(
    adjudication_path: Path,
    annotation_a_path: Path,
    annotation_b_path: Path,
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    validate_adjudication(
        adjudication_path,
        annotation_a_path,
        annotation_b_path,
        collection_path,
        manifest_path,
        rules_reference,
        zoo_manifest,
    )
    adjudication = _load_adjudication(adjudication_path)
    collection = load_collection(collection_path, manifest_path, rules_reference, zoo_manifest)
    observations = {
        case["id"]: case for case in collection["cases"] if isinstance(case, dict)
    }
    cases: list[dict[str, object]] = []
    counts = {key: 0 for key in ("tp", "fn", "tn", "fp", "excluded")}
    for labeled in adjudication["case"]:
        case_id = labeled["id"]
        observation = observations.get(case_id)
        if observation is None:
            raise HoldoutManifestError(f"collection is missing case {case_id}")
        observed_rules = sorted(set(observation["review"].get("in_diff_rule_ids", [])))
        expected_rules = sorted(set(labeled["expected_rule_ids"]))
        outcome = _case_outcome(labeled["adjudicated"], expected_rules, observed_rules)
        counts[outcome] += 1
        cases.append(
            {
                "id": case_id,
                "gold_label": labeled["adjudicated"],
                "expected_rule_ids": expected_rules,
                "observed_in_diff_rule_ids": observed_rules,
                "outcome": outcome,
            }
        )
    positive_predictions = counts["tp"] + counts["fp"]
    actual_positives = counts["tp"] + counts["fn"]
    actual_negatives = counts["tn"] + counts["fp"]
    return {
        "schema_version": METRICS_SCHEMA_VERSION,
        "corpus": adjudication["corpus"],
        "protocol": adjudication["protocol"],
        "manifest_sha256": adjudication["manifest_sha256"],
        "collection_sha256": adjudication["collection_sha256"],
        "adjudication_sha256": sha256_file(adjudication_path),
        "scope": "adjudicated real-history holdout cases only",
        "limitation": "descriptive corpus evidence; not a production or language-wide estimate",
        "cases": cases,
        "counts": counts,
        "metrics": {
            "recall": _metric(counts["tp"], actual_positives),
            "specificity": _metric(counts["tn"], actual_negatives),
            "precision": _metric(counts["tp"], positive_predictions),
            "case_coverage": _metric(actual_positives + actual_negatives, len(cases)),
        },
    }


def write_metrics(
    output_path: Path,
    adjudication_path: Path,
    annotation_a_path: Path,
    annotation_b_path: Path,
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    result = build_metrics(
        adjudication_path,
        annotation_a_path,
        annotation_b_path,
        collection_path,
        manifest_path,
        rules_reference,
        zoo_manifest,
    )
    output_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result
