"""Score a single-expert real-history contract pilot as exploratory evidence."""

from __future__ import annotations

import hashlib
import json
import math
from pathlib import Path
from typing import Any

from real_history_contract import HoldoutManifestError
from real_history_contract_pilot import (
    PILOT_PROTOCOL,
    _cases_by_id,
    _load_toml,
    validate_contract_pilot,
)
from real_history_contracts import CONTRACT_IDS


def _load_collection_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HoldoutManifestError(f"cannot read collection artifact {path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError("collection artifact must be a JSON object")
    return data


def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _metric(successes: int, trials: int) -> dict[str, object]:
    if trials < 0 or successes < 0 or successes > trials:
        raise HoldoutManifestError("invalid contract pilot metric counts")
    if trials == 0:
        interval = None
    else:
        z = 1.96
        proportion = successes / trials
        denominator = 1 + z * z / trials
        center = (proportion + z * z / (2 * trials)) / denominator
        margin = z * math.sqrt(proportion * (1 - proportion) / trials + z * z / (4 * trials * trials)) / denominator
        interval = [max(0.0, center - margin), min(1.0, center + margin)]
    return {
        "successes": successes,
        "trials": trials,
        "value": successes / trials if trials else None,
        "wilson_95": interval,
    }


def _outcome(expected: bool, observed: bool, excluded: bool) -> str:
    if excluded:
        return "excluded"
    if expected and observed:
        return "tp"
    if expected:
        return "fn"
    if observed:
        return "fp"
    return "tn"


def build_contract_pilot_metrics(
    pilot_path: Path,
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    validation = validate_contract_pilot(pilot_path, collection_path, manifest_path, rules_reference, zoo_manifest)
    pilot = _load_toml(pilot_path)
    collection = _load_collection_json(collection_path)
    observations = _cases_by_id(collection)
    raw_cases = {case["id"]: case for case in pilot["case"]}
    counts = {key: 0 for key in ("tp", "fn", "tn", "fp", "excluded")}
    contract_counts = {contract_id: dict(counts) for contract_id in CONTRACT_IDS}
    case_rows: list[dict[str, object]] = []
    observed_counts = {contract_id: 0 for contract_id in CONTRACT_IDS}
    for case_id, labeled in raw_cases.items():
        observation = observations[case_id]
        observed = sorted(set(observation["review"]["contract_delta_ids"]) & set(CONTRACT_IDS))
        expected = sorted(set(labeled["expected_contract_ids"]))
        label = labeled["contract_label"]
        case_outcome = (
            "excluded" if label == "uncertain" else
            "tp" if label == "contract-present" and set(expected) <= set(observed) else
            "fn" if label == "contract-present" else
            "fp" if observed else "tn"
        )
        counts[case_outcome] += 1
        for contract_id in observed:
            observed_counts[contract_id] += 1
        for contract_id in CONTRACT_IDS:
            contract_counts[contract_id][_outcome(
                contract_id in expected, contract_id in observed, label == "uncertain"
            )] += 1
        case_rows.append({
            "id": case_id,
            "contract_label": label,
            "expected_contract_ids": expected,
            "observed_contract_ids": observed,
            "outcome": case_outcome,
        })
    contract_metrics: dict[str, object] = {}
    for contract_id, result in contract_counts.items():
        actual_positives = result["tp"] + result["fn"]
        actual_negatives = result["tn"] + result["fp"]
        predicted_positives = result["tp"] + result["fp"]
        contract_metrics[contract_id] = {
            "recall": _metric(result["tp"], actual_positives),
            "specificity": _metric(result["tn"], actual_negatives),
            "precision": _metric(result["tp"], predicted_positives),
        }
    actual_positives = counts["tp"] + counts["fn"]
    actual_negatives = counts["tn"] + counts["fp"]
    return {
        "schema_version": 1,
        "corpus": pilot["corpus"],
        "protocol": PILOT_PROTOCOL,
        "scope": validation["scope"],
        "limitation": "single reviewer and bounded measured contract subset; not independent validation or a production estimate",
        "manifest_sha256": _sha256_file(manifest_path),
        "collection_sha256": _sha256_file(collection_path),
        "pilot_sha256": _sha256_file(pilot_path),
        "reviewer": pilot["reviewer"],
        "contract_ids": list(CONTRACT_IDS),
        "cases": case_rows,
        "counts": counts,
        "contract_counts": contract_counts,
        "contract_metrics": contract_metrics,
        "observed_contract_counts": observed_counts,
        "metrics": {
            "recall": _metric(counts["tp"], actual_positives),
            "specificity": _metric(counts["tn"], actual_negatives),
            "precision": _metric(counts["tp"], counts["tp"] + counts["fp"]),
            "case_coverage": _metric(actual_positives + actual_negatives, len(case_rows)),
        },
    }


def write_contract_pilot_metrics(
    output_path: Path,
    pilot_path: Path,
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    result = build_contract_pilot_metrics(
        pilot_path, collection_path, manifest_path, rules_reference, zoo_manifest
    )
    output_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def validate_contract_pilot_metrics(
    metrics_path: Path,
    pilot_path: Path,
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    try:
        actual = json.loads(metrics_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HoldoutManifestError(f"cannot read contract pilot metrics {metrics_path}: {error}") from error
    if not isinstance(actual, dict):
        raise HoldoutManifestError("contract pilot metrics must be a JSON object")
    expected = build_contract_pilot_metrics(
        pilot_path, collection_path, manifest_path, rules_reference, zoo_manifest
    )
    if actual != expected:
        raise HoldoutManifestError("contract pilot metrics artifact does not match recomputed score")
    return {
        "status": "valid",
        "protocol": expected["protocol"],
        "scope": expected["scope"],
        "reviewer": expected["reviewer"],
        "cases": len(expected["cases"]),
        "contract_ids": expected["contract_ids"],
    }
