"""Audit label coverage and unresolved contract observations in a holdout."""

from __future__ import annotations

import json
import hashlib
import tomllib
from pathlib import Path
from typing import Any

from real_history_adjudication import validate_adjudication
from real_history_annotations import load_collection
from real_history_contract import HoldoutManifestError, validate_manifest
from real_history_contract_pilot import validate_contract_pilot
from real_history_contracts import ALL_CONTRACT_IDS, CONTRACT_IDS


COVERAGE_SCHEMA_VERSION = 1


def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _load_toml(path: Path, name: str) -> dict[str, Any]:
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise HoldoutManifestError(f"cannot read {name} {path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError(f"{name} must be a TOML table")
    return data


def _source_records(
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
    adjudication_path: Path | None,
    annotation_a_path: Path | None,
    annotation_b_path: Path | None,
    pilot_path: Path | None,
) -> tuple[str, dict[str, dict[str, Any]]]:
    dual = (adjudication_path, annotation_a_path, annotation_b_path)
    if pilot_path is not None and any(value is not None for value in dual):
        raise HoldoutManifestError("coverage audit accepts either a pilot or a complete dual packet")
    if pilot_path is not None:
        validate_contract_pilot(pilot_path, collection_path, manifest_path, rules_reference, zoo_manifest)
        data = _load_toml(pilot_path, "contract pilot")
        return "single-expert-pilot", {
            case["id"]: {
                "label": case["contract_label"],
                "expected_contract_ids": sorted(case["expected_contract_ids"]),
            }
            for case in data["case"]
        }
    if any(value is not None for value in dual):
        if any(value is None for value in dual):
            raise HoldoutManifestError("coverage audit requires annotation, annotation-a, and annotation-b together")
        assert adjudication_path is not None
        assert annotation_a_path is not None
        assert annotation_b_path is not None
        validate_adjudication(
            adjudication_path,
            annotation_a_path,
            annotation_b_path,
            collection_path,
            manifest_path,
            rules_reference,
            zoo_manifest,
        )
        data = _load_toml(adjudication_path, "adjudication")
        return "dual-adjudication", {
            case["id"]: {
                "label": case["adjudicated"],
                "expected_contract_ids": sorted(case["expected_contract_ids"]),
            }
            for case in data["case"]
        }
    return "pending", {}


def _case_rows(
    collection: dict[str, Any],
    labels: dict[str, dict[str, Any]],
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], set[str]]:
    rows: list[dict[str, Any]] = []
    unreviewed: list[dict[str, Any]] = []
    observed_ids: set[str] = set()
    for case in sorted(collection["cases"], key=lambda item: item["id"]):
        review = case["review"]
        observed = sorted(set(review.get("contract_delta_ids", [])))
        observed_ids.update(observed)
        measured_observed = sorted(set(observed) & set(CONTRACT_IDS))
        label = labels.get(case["id"])
        expected = sorted(label["expected_contract_ids"]) if label else []
        if label is None:
            label_state = "unlabeled"
            observed_without_expected: list[str] = []
            expected_without_observation: list[str] = []
            if measured_observed:
                unreviewed.append({"case_id": case["id"], "contract_ids": measured_observed})
        else:
            label_state = "uncertain" if label["label"] == "uncertain" else "labeled"
            observed_without_expected = sorted(set(measured_observed) - set(expected))
            expected_without_observation = sorted(set(expected) - set(measured_observed))
        rows.append(
            {
                "id": case["id"],
                "label_state": label_state,
                "label": label["label"] if label else None,
                "observed_contract_ids": observed,
                "measured_observed_contract_ids": measured_observed,
                "unmeasured_observed_contract_ids": sorted(set(observed) - set(CONTRACT_IDS)),
                "expected_contract_ids": expected,
                "observed_without_expected_ids": observed_without_expected,
                "expected_without_observation_ids": expected_without_observation,
            }
        )
    return rows, unreviewed, observed_ids


def _contract_coverage(rows: list[dict[str, Any]], labels_count: int) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for contract_id in ALL_CONTRACT_IDS:
        observed_cases = [row for row in rows if contract_id in row["observed_contract_ids"]]
        expected_cases = [row for row in rows if contract_id in row["expected_contract_ids"]]
        unreviewed_cases = [row for row in observed_cases if row["label_state"] == "unlabeled"]
        result[contract_id] = {
            "measured": contract_id in CONTRACT_IDS,
            "unmeasured": contract_id not in CONTRACT_IDS,
            "observed_cases": len(observed_cases),
            "observed_labeled_cases": len(observed_cases) - len(unreviewed_cases),
            "expected_cases": len(expected_cases),
            "unreviewed_cases": len(unreviewed_cases),
            "labels_available": labels_count,
        }
    return result


def build_label_coverage(
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
    *,
    adjudication_path: Path | None = None,
    annotation_a_path: Path | None = None,
    annotation_b_path: Path | None = None,
    pilot_path: Path | None = None,
) -> dict[str, Any]:
    collection = load_collection(collection_path, manifest_path, rules_reference, zoo_manifest)
    _, _, manifest_cases = validate_manifest(manifest_path, rules_reference, zoo_manifest)
    source, labels = _source_records(
        collection_path,
        manifest_path,
        rules_reference,
        zoo_manifest,
        adjudication_path,
        annotation_a_path,
        annotation_b_path,
        pilot_path,
    )
    rows, unreviewed, observed_ids = _case_rows(collection, labels)
    if source != "pending" and set(labels) != {case.case_id for case in manifest_cases}:
        raise HoldoutManifestError("label packet does not cover the manifest cases")
    limitation = {
        "pending": "no human labels supplied; observed contract deltas remain unresolved",
        "single-expert-pilot": "one reviewer only; coverage is exploratory and not independent validation",
        "dual-adjudication": "coverage compares a bounded holdout packet; it does not prove runtime semantics",
    }[source]
    cases_labeled = len(labels)
    label_inputs: dict[str, str] = {}
    if pilot_path is not None:
        label_inputs["pilot"] = _sha256_file(pilot_path)
    elif adjudication_path is not None:
        label_inputs = {
            "adjudication": _sha256_file(adjudication_path),
            "annotation_a": _sha256_file(annotation_a_path),
            "annotation_b": _sha256_file(annotation_b_path),
        }
    return {
        "schema_version": COVERAGE_SCHEMA_VERSION,
        "corpus": collection["corpus"],
        "protocol": "real-history-label-coverage-audit-v1",
        "label_source": source,
        "independent_validation": source == "dual-adjudication",
        "ready_for_dual_metrics": source == "dual-adjudication",
        "limitation": limitation,
        "manifest_sha256": collection["manifest_sha256"],
        "collection_sha256": collection["collection_sha256"],
        "label_inputs": label_inputs,
        "cases_total": len(rows),
        "cases_labeled": cases_labeled,
        "cases_unlabeled": len(rows) - cases_labeled,
        "cases_uncertain": sum(row["label_state"] == "uncertain" for row in rows),
        "observed_contract_ids": sorted(observed_ids),
        "unmeasured_observed_contract_ids": sorted(observed_ids - set(CONTRACT_IDS)),
        "unreviewed_observations": unreviewed,
        "contract_coverage": _contract_coverage(rows, cases_labeled),
        "cases": rows,
    }


def write_label_coverage(
    output_path: Path,
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
    **sources: Path | None,
) -> dict[str, Any]:
    result = build_label_coverage(
        collection_path,
        manifest_path,
        rules_reference,
        zoo_manifest,
        **sources,
    )
    output_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def validate_label_coverage(
    coverage_path: Path,
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
    **sources: Path | None,
) -> dict[str, Any]:
    try:
        actual = json.loads(coverage_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HoldoutManifestError(f"cannot read label coverage artifact {coverage_path}: {error}") from error
    if not isinstance(actual, dict):
        raise HoldoutManifestError("label coverage artifact must be a JSON object")
    expected = build_label_coverage(
        collection_path,
        manifest_path,
        rules_reference,
        zoo_manifest,
        **sources,
    )
    if actual != expected:
        raise HoldoutManifestError("label coverage artifact does not match recomputed audit")
    return {
        "status": "valid",
        "label_source": expected["label_source"],
        "cases": expected["cases_total"],
        "unreviewed_observations": len(expected["unreviewed_observations"]),
    }
