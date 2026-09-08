"""Validate repeated differential utility observations without scoring them."""

from __future__ import annotations

import hashlib
import json
from collections import Counter
from pathlib import Path
from typing import Any

from differential_contract import validate_differential
from differential_identity import REVIEW_COMPARISON_SCHEME
from differential_manifest import DifferentialManifestError
from differential_telemetry import validate_telemetry
from real_history_contract import HoldoutManifestError, validate_manifest
from real_history_runner import BASELINE_COMMANDS


REVIEW_STATUSES = {"collected", "timeout"}
BASE_SCAN_STATUSES = REVIEW_STATUSES


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_data(
    data: dict[str, Any],
    manifest_path: Path,
    differential_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    protocol = validate_differential(differential_path, manifest_path, rules_reference, zoo_manifest)
    schema_version = _validate_header(data, protocol, manifest_path, differential_path)
    _validate_review_verification_config(data)
    observations = data.get("cases")
    if not isinstance(observations, list):
        raise DifferentialManifestError("differential artifact cases must be an array")
    _, _, holdout_cases = validate_manifest(manifest_path, rules_reference, zoo_manifest)
    expected = {case.case_id: case for case in holdout_cases}
    differential_cases = {case.case_id: case for case in _load_cases(differential_path)}
    _validate_observations(observations, expected, differential_cases, data["repetitions"], schema_version)
    baseline_evidence = baseline_evidence_summary(observations)
    return {
        "status": "valid",
        "schema_version": schema_version,
        "corpus": protocol["corpus"],
        "protocol": protocol["protocol"],
        "cases": len(observations),
        "baseline_observations": sum(
            sum(len(runs) for runs in observation["baselines"].values())
            for observation in observations
        ),
        "review_observations": sum(len(observation["reviews"]) for observation in observations),
        "baseline_evidence": baseline_evidence,
    }


def baseline_evidence_summary(observations: list[dict[str, Any]]) -> dict[str, object]:
    """Summarize adapter coverage without turning missing evidence into a pass."""

    total = tracked = measured = unavailable = untracked = key_count = 0
    comparison_measured = comparison_unavailable = comparison_untracked = comparison_key_count = 0
    sources: Counter[str] = Counter()
    comparison_by_baseline: dict[str, dict[str, object]] = {}
    comparison_reasons: Counter[str] = Counter()
    verification_measured = verification_unavailable = verification_untracked = verification_keys = 0
    verification_reasons: Counter[str] = Counter()
    for observation in observations:
        for review in observation.get("reviews", []):
            verification = review.get("verification_comparison") if isinstance(review, dict) else None
            if not isinstance(verification, dict):
                verification_untracked += 1
            elif verification.get("status") == "measured":
                verification_measured += 1
                verification_keys += len(verification.get("keys", []))
            elif verification.get("status") == "unavailable":
                verification_unavailable += 1
                verification_reasons[str(verification.get("reason") or "unspecified")] += 1
        for baseline_id, runs in observation.get("baselines", {}).items():
            coverage = comparison_by_baseline.setdefault(
                baseline_id,
                {
                    "total": 0,
                    "measured": 0,
                    "unavailable": 0,
                    "untracked": 0,
                    "keys": 0,
                    "unavailable_reasons": Counter(),
                },
            )
            for run in runs:
                total += 1
                coverage["total"] += 1
                evidence = run.get("evidence") if isinstance(run, dict) else None
                if not isinstance(evidence, dict):
                    untracked += 1
                    comparison_untracked += 1
                    coverage["untracked"] += 1
                    continue
                tracked += 1
                status = evidence.get("status")
                if status == "measured":
                    measured += 1
                    key_count += len(evidence.get("keys", []))
                    source = evidence.get("source")
                    sources[str(source) if source else "<unspecified>"] += 1
                elif status == "unavailable":
                    unavailable += 1
                comparison = evidence.get("comparison")
                if isinstance(comparison, dict) and comparison.get("status") == "measured":
                    comparison_measured += 1
                    keys = comparison.get("keys", [])
                    comparison_key_count += len(keys)
                    coverage["measured"] += 1
                    coverage["keys"] += len(keys)
                elif isinstance(comparison, dict) and comparison.get("status") == "unavailable":
                    comparison_unavailable += 1
                    coverage["unavailable"] += 1
                    reason = str(comparison.get("reason") or "unspecified")
                    comparison_reasons[reason] += 1
                    coverage["unavailable_reasons"][reason] += 1
                else:
                    comparison_untracked += 1
                    coverage["untracked"] += 1
    for coverage in comparison_by_baseline.values():
        reasons = coverage["unavailable_reasons"]
        coverage["unavailable_reasons"] = dict(sorted(reasons.items()))
        denominator = coverage["measured"] + coverage["unavailable"]
        coverage["measurement_rate"] = round(coverage["measured"] / denominator, 3) if denominator else None
    return {
        "total": total,
        "tracked": tracked,
        "measured": measured,
        "unavailable": unavailable,
        "untracked": untracked,
        "tracked_rate": round(tracked / total, 3) if total else None,
        "measurement_rate": round(measured / tracked, 3) if tracked else None,
        "keys": key_count,
        "sources": dict(sorted(sources.items())),
        "comparison": {
            "measured": comparison_measured,
            "unavailable": comparison_unavailable,
            "untracked": comparison_untracked,
            "keys": comparison_key_count,
            "measurement_rate": round(
                comparison_measured / (comparison_measured + comparison_unavailable), 3
            )
            if comparison_measured + comparison_unavailable
            else None,
            "by_baseline": dict(sorted(comparison_by_baseline.items())),
            "unavailable_reasons": dict(sorted(comparison_reasons.items())),
        },
        "review_verification": {
            "measured": verification_measured,
            "unavailable": verification_unavailable,
            "untracked": verification_untracked,
            "keys": verification_keys,
            "unavailable_reasons": dict(sorted(verification_reasons.items())),
        },
    }


def _validate_header(
    data: dict[str, Any], protocol: dict[str, Any], manifest_path: Path, differential_path: Path
) -> int:
    schema_version = data.get("schema_version")
    if schema_version not in {1, 2}:
        raise DifferentialManifestError("differential artifact schema_version is unsupported")
    if data.get("corpus") != protocol["corpus"] or data.get("protocol") != protocol["protocol"]:
        raise DifferentialManifestError("differential artifact corpus/protocol does not match manifest")
    if data.get("manifest_sha256") != sha256_file(manifest_path):
        raise DifferentialManifestError("differential artifact manifest_sha256 does not match manifest")
    if data.get("differential_manifest_sha256") != sha256_file(differential_path):
        raise DifferentialManifestError("differential artifact differential_manifest_sha256 does not match manifest")
    if data.get("repetitions") != protocol["repetitions"]:
        raise DifferentialManifestError("differential artifact repetitions do not match manifest")
    if data.get("label_state") != "pending":
        raise DifferentialManifestError("differential artifact must remain unlabeled")
    scanner = data.get("scanner")
    if not isinstance(scanner, dict):
        raise DifferentialManifestError("differential artifact is missing scanner provenance")
    for field in ("mode", "version", "report_schema_version", "workspace_version"):
        if not isinstance(scanner.get(field), str) or not scanner[field]:
            raise DifferentialManifestError(f"scanner provenance missing {field}")
    return schema_version


def _validate_observations(
    observations: list[Any],
    expected: dict[str, Any],
    differential_cases: dict[str, Any],
    repetitions: int,
    schema_version: int,
) -> None:
    observed: set[str] = set()
    for observation in observations:
        if not isinstance(observation, dict):
            raise DifferentialManifestError("differential case must be an object")
        case_id = observation.get("id")
        if not isinstance(case_id, str) or case_id in observed or case_id not in expected:
            raise DifferentialManifestError(f"differential artifact has unknown or duplicate case {case_id!r}")
        observed.add(case_id)
        validate_case(
            observation,
            expected[case_id],
            differential_cases[case_id].baseline_ids,
            repetitions,
            schema_version,
        )
    if observed != set(expected):
        missing = ", ".join(sorted(set(expected) - observed))
        raise DifferentialManifestError(f"differential artifact is missing cases: {missing}")


def _load_cases(path: Path) -> list[Any]:
    from differential_manifest import load_differential

    return load_differential(path)[4]


def validate_case(
    observation: dict[str, Any],
    case: Any,
    baseline_ids: tuple[str, ...],
    repetitions: int,
    schema_version: int = 1,
) -> None:
    for field in ("repo", "base_sha", "head_sha", "merge_sha"):
        if observation.get(field) != getattr(case, field):
            raise DifferentialManifestError(f"case {case.case_id}: {field} does not match holdout manifest")
    baselines = observation.get("baselines")
    if not isinstance(baselines, dict) or set(baselines) != set(baseline_ids):
        raise DifferentialManifestError(f"case {case.case_id}: baseline set does not match differential manifest")
    for baseline_id, runs in baselines.items():
        if not isinstance(runs, list) or len(runs) != repetitions:
            raise DifferentialManifestError(f"case {case.case_id}: baseline {baseline_id} repetition count drift")
        for run in runs:
            if not isinstance(run, dict) or run.get("command") != list(BASELINE_COMMANDS[baseline_id]):
                raise DifferentialManifestError(f"case {case.case_id}: baseline command drift for {baseline_id}")
            if run.get("status") not in {"passed", "failed", "unavailable", "timeout"}:
                raise DifferentialManifestError(f"case {case.case_id}: invalid baseline status")
            if not isinstance(run.get("wall_ms"), (int, float)) or run["wall_ms"] < 0:
                raise DifferentialManifestError(f"case {case.case_id}: invalid baseline timing")
            _validate_run_telemetry(run, run["wall_ms"], schema_version, f"case {case.case_id} baseline")
            _validate_resource_usage(run, f"case {case.case_id} baseline")
            _validate_resource_phase(run, f"case {case.case_id} baseline")
            _validate_baseline_evidence(run, f"case {case.case_id} baseline", schema_version, baseline_id)
    base_scan = observation.get("base_scan")
    if not isinstance(base_scan, dict) or base_scan.get("status") not in BASE_SCAN_STATUSES:
        raise DifferentialManifestError(f"case {case.case_id}: base scan observation is missing")
    if not isinstance(base_scan.get("wall_ms"), (int, float)) or base_scan["wall_ms"] < 0:
        raise DifferentialManifestError(f"case {case.case_id}: invalid base scan timing")
    _validate_run_telemetry(base_scan, base_scan["wall_ms"], schema_version, f"case {case.case_id} base scan")
    if base_scan["status"] == "collected" and not isinstance(base_scan.get("evidence_keys"), list):
        raise DifferentialManifestError(f"case {case.case_id}: base scan evidence keys are missing")
    reviews = observation.get("reviews")
    if not isinstance(reviews, list) or len(reviews) != repetitions:
        raise DifferentialManifestError(f"case {case.case_id}: review repetition count drift")
    for review in reviews:
        if not isinstance(review, dict) or review.get("status") not in REVIEW_STATUSES:
            raise DifferentialManifestError(f"case {case.case_id}: invalid review status")
        if not isinstance(review.get("wall_ms"), (int, float)) or review["wall_ms"] < 0:
            raise DifferentialManifestError(f"case {case.case_id}: invalid review timing")
        _validate_run_telemetry(review, review["wall_ms"], schema_version, f"case {case.case_id} review")
        _validate_resource_usage(review, f"case {case.case_id} review")
        _validate_resource_phase(review, f"case {case.case_id} review")
        if review["status"] == "collected" and not isinstance(review.get("stable_evidence_sha256"), str):
            raise DifferentialManifestError(f"case {case.case_id}: collected review lacks stable evidence hash")
        if review["status"] == "collected":
            for field in ("in_diff_evidence_keys", "novel_in_diff_evidence_keys"):
                if not isinstance(review.get(field), list) or not all(isinstance(item, str) for item in review[field]):
                    raise DifferentialManifestError(f"case {case.case_id}: collected review lacks {field}")
            comparison_keys = review.get("in_diff_comparison_keys", [])
            if not isinstance(comparison_keys, list) or not all(
                isinstance(item, str) for item in comparison_keys
            ):
                raise DifferentialManifestError(
                    f"case {case.case_id}: collected review has invalid comparison keys"
                )
            _validate_review_verification(review, f"case {case.case_id} review")
    determinism = observation.get("determinism")
    if not isinstance(determinism, dict) or not isinstance(determinism.get("stable_evidence_deterministic"), bool):
        raise DifferentialManifestError(f"case {case.case_id}: determinism summary is missing")


def _validate_run_telemetry(
    run: dict[str, Any], wall_ms: float, schema_version: int, context: str
) -> None:
    telemetry = run.get("telemetry")
    if telemetry is None and schema_version == 1:
        return
    try:
        validate_telemetry(telemetry, wall_ms, context)
    except ValueError as error:
        raise DifferentialManifestError(str(error)) from error


def _validate_resource_usage(run: dict[str, Any], context: str) -> None:
    status = run.get("resource_status")
    if status is None:
        return
    source = run.get("resource_source")
    if source is not None and (not isinstance(source, str) or not source):
        raise DifferentialManifestError(f"{context}: resource source is invalid")
    if status not in {"available", "unavailable"}:
        raise DifferentialManifestError(f"{context}: resource status is invalid")
    sample = run.get("child_max_rss_kb")
    if status == "available":
        if not isinstance(sample, (int, float)) or sample < 0:
            raise DifferentialManifestError(f"{context}: available resource sample is missing")
        if "resource_reason" in run:
            raise DifferentialManifestError(f"{context}: available resource sample has a reason")
        if source == "unavailable":
            raise DifferentialManifestError(f"{context}: available resource sample has an unavailable source")
    elif not isinstance(run.get("resource_reason"), str) or not run["resource_reason"]:
        raise DifferentialManifestError(f"{context}: unavailable resource reason is missing")
    if status == "unavailable" and sample is not None:
        raise DifferentialManifestError(f"{context}: unavailable resource sample must be omitted")


def _validate_resource_phase(run: dict[str, Any], context: str) -> None:
    phase = run.get("resource_phase")
    if phase is not None and phase not in {"cold", "warm"}:
        raise DifferentialManifestError(f"{context}: resource phase is invalid")


def _validate_baseline_evidence(
    run: dict[str, Any], context: str, schema_version: int = 1, baseline_id: str | None = None
) -> None:
    evidence = run.get("evidence")
    if evidence is None:
        return
    if not isinstance(evidence, dict) or evidence.get("status") not in {"measured", "unavailable"}:
        raise DifferentialManifestError(f"{context}: baseline evidence status is invalid")
    if evidence["status"] == "measured":
        keys = evidence.get("keys")
        if not isinstance(keys, list) or not all(isinstance(key, str) for key in keys):
            raise DifferentialManifestError(f"{context}: measured baseline evidence keys are missing")
        if schema_version >= 2:
            source = evidence.get("source")
            if not isinstance(source, str) or not source:
                raise DifferentialManifestError(f"{context}: measured baseline evidence source is missing")
            expected_source = f"{baseline_id}-v1" if baseline_id else None
            if expected_source and source != expected_source:
                raise DifferentialManifestError(f"{context}: measured baseline evidence source drift")
            comparison = evidence.get("comparison")
            if not isinstance(comparison, dict) or comparison.get("status") not in {"measured", "unavailable"}:
                raise DifferentialManifestError(f"{context}: baseline evidence comparison is missing")
            if comparison.get("scheme") != REVIEW_COMPARISON_SCHEME:
                raise DifferentialManifestError(f"{context}: baseline evidence comparison scheme drift")
            if comparison["status"] == "measured":
                comparison_keys = comparison.get("keys")
                if not isinstance(comparison_keys, list) or not all(
                    isinstance(key, str) for key in comparison_keys
                ):
                    raise DifferentialManifestError(f"{context}: measured comparison keys are missing")
            elif not isinstance(comparison.get("reason"), str) or not comparison["reason"]:
                raise DifferentialManifestError(f"{context}: unavailable comparison reason is missing")
    elif not isinstance(evidence.get("reason"), str) or not evidence["reason"]:
        raise DifferentialManifestError(f"{context}: unavailable baseline evidence reason is missing")


def _validate_review_verification(review: dict[str, Any], context: str) -> None:
    """Validate optional exact pytest evidence emitted by explicit review verification."""

    evidence = review.get("verification_comparison")
    if evidence is None:
        return
    if not isinstance(evidence, dict) or evidence.get("status") not in {"measured", "unavailable"}:
        raise DifferentialManifestError(f"{context}: review verification comparison status is invalid")
    if evidence["status"] == "measured":
        keys = evidence.get("keys")
        if not isinstance(keys, list) or not all(isinstance(key, str) for key in keys):
            raise DifferentialManifestError(f"{context}: review verification comparison keys are missing")
        if evidence.get("scheme") != "review-verification-v1":
            raise DifferentialManifestError(f"{context}: review verification comparison scheme drift")
    elif not isinstance(evidence.get("reason"), str) or not evidence["reason"]:
        raise DifferentialManifestError(f"{context}: review verification comparison reason is missing")


def _validate_review_verification_config(data: dict[str, Any]) -> None:
    config = data.get("review_verification")
    if config is None:
        return
    if not isinstance(config, dict):
        raise DifferentialManifestError("differential artifact review verification config is invalid")
    checks = config.get("checks")
    if not isinstance(checks, list) or not all(isinstance(check, str) for check in checks):
        raise DifferentialManifestError("differential artifact review verification checks are invalid")
    if len(set(checks)) != len(checks) or any(check != "python.tests" for check in checks):
        raise DifferentialManifestError("differential artifact review verification checks are unsupported")
    config_hash = config.get("config_sha256")
    if config_hash is not None and (
        not isinstance(config_hash, str)
        or len(config_hash) != 64
        or any(character not in "0123456789abcdef" for character in config_hash)
    ):
        raise DifferentialManifestError("differential artifact review verification config hash is invalid")
    if checks and config_hash is None:
        raise DifferentialManifestError("differential artifact review verification config hash is missing")


def validate_artifact(
    artifact_path: Path,
    manifest_path: Path,
    differential_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
) -> dict[str, object]:
    try:
        data = json.loads(artifact_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise HoldoutManifestError(f"cannot read differential artifact {artifact_path}: {error}") from error
    if not isinstance(data, dict):
        raise HoldoutManifestError("differential artifact must be a JSON object")
    return validate_data(data, manifest_path, differential_path, rules_reference, zoo_manifest)
