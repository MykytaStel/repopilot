"""Build and validate the third-party decision artifact for a holdout case."""

from __future__ import annotations

import re
from pathlib import Path

from real_history_annotations import (
    ANNOTATION_SCHEMA_VERSION,
    _toml_array,
    _toml_string,
    load_annotation,
    load_collection,
    _load_toml,
)
from real_history_contract import ALLOWED_LABELS, HoldoutManifestError, validate_manifest
from real_history_contracts import ContractEvidenceError, validate_expected_contract_ids


ADJUDICATION_FIELDS = {
    "id",
    "label_a",
    "expected_rule_ids_a",
    "expected_contract_ids_a",
    "label_b",
    "expected_rule_ids_b",
    "expected_contract_ids_b",
    "adjudicated",
    "expected_rule_ids",
    "expected_contract_ids",
    "rationale",
}


def render_adjudication_template(
    annotation_a_path: Path,
    annotation_b_path: Path,
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> str:
    data_a, cases_a = load_annotation(
        annotation_a_path, collection_path, manifest_path, rules_reference, zoo_manifest, "a"
    )
    data_b, cases_b = load_annotation(
        annotation_b_path, collection_path, manifest_path, rules_reference, zoo_manifest, "b"
    )
    lines = [
        f"schema_version = {ANNOTATION_SCHEMA_VERSION}",
        f"corpus = {_toml_string(data_a['corpus'])}",
        f"protocol = {_toml_string(data_a['protocol'])}",
        f"manifest_sha256 = {_toml_string(data_a['manifest_sha256'])}",
        f"collection_sha256 = {_toml_string(data_a['collection_sha256'])}",
        "blinded = true",
        "",
    ]
    for case_id in sorted(cases_a):
        left, right = cases_a[case_id], cases_b[case_id]
        lines.extend([
            "[[case]]",
            f"id = {_toml_string(case_id)}",
            f"label_a = {_toml_string(left['label'])}",
            f"expected_rule_ids_a = {_toml_array(left['expected_rule_ids'])}",
            f"expected_contract_ids_a = {_toml_array(left['expected_contract_ids'])}",
            f"label_b = {_toml_string(right['label'])}",
            f"expected_rule_ids_b = {_toml_array(right['expected_rule_ids'])}",
            f"expected_contract_ids_b = {_toml_array(right['expected_contract_ids'])}",
            'adjudicated = ""',
            "expected_rule_ids = []",
            "expected_contract_ids = []",
            'rationale = ""',
            "",
        ])
    return "\n".join(lines)


def validate_adjudication(
    adjudication_path: Path,
    annotation_a_path: Path,
    annotation_b_path: Path,
    collection_path: Path,
    manifest_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> int:
    data_a, cases_a = load_annotation(
        annotation_a_path, collection_path, manifest_path, rules_reference, zoo_manifest, "a"
    )
    data_b, cases_b = load_annotation(
        annotation_b_path, collection_path, manifest_path, rules_reference, zoo_manifest, "b"
    )
    collection = load_collection(collection_path, manifest_path, rules_reference, zoo_manifest)
    _, _, manifest_cases = validate_manifest(manifest_path, rules_reference, zoo_manifest)
    known_rules = set(re.findall(r"^### `([^`]+)`", rules_reference.read_text(encoding="utf-8"), re.MULTILINE))
    data = _load_toml(adjudication_path, "adjudication")
    if data.get("schema_version") != ANNOTATION_SCHEMA_VERSION:
        raise HoldoutManifestError("adjudication schema_version is unsupported")
    if data.get("blinded") is not True:
        raise HoldoutManifestError("adjudication must declare blinded = true")
    for field in ("corpus", "protocol", "manifest_sha256", "collection_sha256"):
        if data.get(field) != collection.get(field):
            raise HoldoutManifestError(f"adjudication {field} does not match collection")
    raw_cases = data.get("case")
    if not isinstance(raw_cases, list):
        raise HoldoutManifestError("adjudication cases must be an array")
    expected_ids = {case.case_id for case in manifest_cases}
    observed: set[str] = set()
    for raw in raw_cases:
        if not isinstance(raw, dict):
            raise HoldoutManifestError("adjudication case must be a table")
        unknown = set(raw) - ADJUDICATION_FIELDS
        if unknown:
            raise HoldoutManifestError(f"adjudication case has unknown fields: {', '.join(sorted(unknown))}")
        case_id = raw.get("id")
        if not isinstance(case_id, str) or case_id in observed or case_id not in expected_ids:
            raise HoldoutManifestError(f"adjudication has unknown or duplicate case {case_id!r}")
        observed.add(case_id)
        for side, source in (("a", cases_a), ("b", cases_b)):
            label_key = f"label_{side}"
            rules_key = f"expected_rule_ids_{side}"
            if raw.get(label_key) != source[case_id]["label"]:
                raise HoldoutManifestError(f"adjudication case {case_id}: {label_key} does not match worksheet")
            if raw.get(rules_key) != source[case_id]["expected_rule_ids"]:
                raise HoldoutManifestError(f"adjudication case {case_id}: {rules_key} does not match worksheet")
            contracts_key = f"expected_contract_ids_{side}"
            if raw.get(contracts_key) != source[case_id]["expected_contract_ids"]:
                raise HoldoutManifestError(f"adjudication case {case_id}: {contracts_key} does not match worksheet")
        label = raw.get("adjudicated")
        if label not in ALLOWED_LABELS:
            raise HoldoutManifestError(f"adjudication case {case_id}: adjudicated label is required")
        rules = raw.get("expected_rule_ids")
        if not isinstance(rules, list) or not all(isinstance(item, str) for item in rules):
            raise HoldoutManifestError(f"adjudication case {case_id}: expected_rule_ids must be a string array")
        if any(rule_id not in known_rules for rule_id in rules):
            raise HoldoutManifestError(f"adjudication case {case_id}: expected_rule_ids contains an unknown rule")
        if label == "defect-present" and not rules:
            raise HoldoutManifestError(f"adjudication case {case_id}: defect-present needs expected_rule_ids")
        try:
            validate_expected_contract_ids(
                raw.get("expected_contract_ids"),
                f"adjudication case {case_id} expected_contract_ids",
            )
        except ContractEvidenceError as error:
            raise HoldoutManifestError(str(error)) from error
        if not isinstance(raw.get("rationale"), str) or not raw["rationale"].strip():
            raise HoldoutManifestError(f"adjudication case {case_id}: rationale is required")
    if observed != expected_ids:
        raise HoldoutManifestError(f"adjudication is missing cases: {', '.join(sorted(expected_ids - observed))}")
    if data_a["manifest_sha256"] != data_b["manifest_sha256"]:
        raise HoldoutManifestError("worksheets do not share the same manifest")
    return len(raw_cases)
