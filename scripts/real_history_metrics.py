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
from real_history_contracts import CONTRACT_IDS, ContractEvidenceError, validate_expected_contract_ids


METRICS_SCHEMA_VERSION = 2


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


def _contract_outcome(expected: bool, observed: bool, excluded: bool) -> str:
    if excluded:
        return "excluded"
    if expected and observed:
        return "tp"
    if expected:
        return "fn"
    if observed:
        return "fp"
    return "tn"


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
    contract_counts = {
        contract_id: {key: 0 for key in ("tp", "fn", "tn", "fp", "excluded")}
        for contract_id in CONTRACT_IDS
    }
    for labeled in adjudication["case"]:
        case_id = labeled["id"]
        observation = observations.get(case_id)
        if observation is None:
            raise HoldoutManifestError(f"collection is missing case {case_id}")
        observed_rules = sorted(set(observation["review"].get("in_diff_rule_ids", [])))
        expected_rules = sorted(set(labeled["expected_rule_ids"]))
        outcome = _case_outcome(labeled["adjudicated"], expected_rules, observed_rules)
        counts[outcome] += 1
        observed_contracts = sorted(set(observation["review"].get("contract_delta_ids", [])))
        try:
            expected_contracts = list(validate_expected_contract_ids(labeled["expected_contract_ids"]))
        except ContractEvidenceError as error:
            raise HoldoutManifestError(str(error)) from error
        for contract_id in CONTRACT_IDS:
            contract_counts[contract_id][_contract_outcome(
                contract_id in expected_contracts,
                contract_id in observed_contracts,
                labeled["adjudicated"] == "uncertain",
            )] += 1
        cases.append(
            {
                "id": case_id,
                "gold_label": labeled["adjudicated"],
                "expected_rule_ids": expected_rules,
                "observed_in_diff_rule_ids": observed_rules,
                "expected_contract_ids": expected_contracts,
                "observed_contract_ids": observed_contracts,
                "outcome": outcome,
            }
        )
    rule_positive_predictions = counts["tp"] + counts["fp"]
    rule_actual_positives = counts["tp"] + counts["fn"]
    rule_actual_negatives = counts["tn"] + counts["fp"]
    contract_metrics = {}
    for contract_id, contract_result in contract_counts.items():
        contract_positive_predictions = contract_result["tp"] + contract_result["fp"]
        contract_actual_positives = contract_result["tp"] + contract_result["fn"]
        contract_actual_negatives = contract_result["tn"] + contract_result["fp"]
        contract_metrics[contract_id] = {
            "recall": _metric(contract_result["tp"], contract_actual_positives),
            "specificity": _metric(contract_result["tn"], contract_actual_negatives),
            "precision": _metric(contract_result["tp"], contract_positive_predictions),
        }
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
        "contract_ids": list(CONTRACT_IDS),
        "contract_counts": contract_counts,
        "contract_metrics": contract_metrics,
        "metrics": {
            "recall": _metric(counts["tp"], rule_actual_positives),
            "specificity": _metric(counts["tn"], rule_actual_negatives),
            "precision": _metric(counts["tp"], rule_positive_predictions),
            "case_coverage": _metric(rule_actual_positives + rule_actual_negatives, len(cases)),
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
