"""Fail-closed validation and reporting for ChangeProof Benchmark v1 artifacts."""

from __future__ import annotations

import hashlib
import json
import math
import re
from pathlib import Path
from typing import Any

from changeproof_benchmark_contract import (
    BenchmarkCase,
    BenchmarkManifestError,
    fixture_source_sha256,
    rationale_sha256,
    validate_manifest,
)
from changeproof_benchmark_proof import BenchmarkProofError, evaluate_normalized_proof, normalized_proof_sha256, validate_normalized_proof


PHASES = ("cold", "warm", "warm")
ARTIFACT_FIELDS = {"schema_version", "protocol", "corpus", "manifest_sha256", "scanner", "cases"}
CASE_FIELDS = {"id", "fixture", "variant", "fixture_source_sha256", "oracle", "runs", "proof_semantic_hashes", "deterministic"}
RUN_FIELDS = {"phase", "status", "returncode", "wall_ms", "resource", "proof", "proof_semantic_sha256", "evaluation"}
SCANNER_FIELDS = {"mode", "version", "report_schema_version", "workspace_version", "workspace_commit", "workspace_dirty", "version_mismatch_allowed"}
SEMVER = re.compile(r"^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$")
SCHEMA_VERSION = re.compile(r"^\d+\.\d+$")
GIT_COMMIT = re.compile(r"^[0-9a-f]{7,64}$")
RESOURCE_SOURCES = {"posix-time-v1", "unavailable"}
RESOURCE_REASONS = {
    "command timed out before peak RSS sampling completed",
    "portable peak RSS sampler is unavailable on this platform",
    "portable peak RSS sampler output is unavailable",
    "portable peak RSS sampler emitted no positive sample",
}


def validate_artifact_file(path: Path, manifest_path: Path, fixture_root: Path) -> dict[str, object]:
    """Recompute all input, oracle, and proof identities before reporting."""

    try:
        artifact = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise BenchmarkManifestError(f"cannot read benchmark artifact: {error}") from error
    return validate_artifact(artifact, manifest_path, fixture_root)


def validate_artifact(artifact: object, manifest_path: Path, fixture_root: Path) -> dict[str, object]:
    """Validate an in-memory collection before it may be persisted or reported."""

    value = _object(artifact, "benchmark artifact")
    _exact_fields(value, ARTIFACT_FIELDS, "benchmark artifact")
    corpus, cases = validate_manifest(manifest_path, fixture_root)
    _validate_header(value, corpus, manifest_path)
    observations = value["cases"]
    if not isinstance(observations, list):
        raise BenchmarkManifestError("benchmark artifact cases must be an array")
    expected, seen, validated = {case.case_id: case for case in cases}, set(), []
    for observation in observations:
        validated.append(_validate_case(observation, expected, fixture_root, seen))
    if seen != set(expected):
        raise BenchmarkManifestError(f"benchmark artifact is missing cases: {', '.join(sorted(set(expected) - seen))}")
    return {"artifact": value, "summary": _summary(validated)}


def render_report(validated: dict[str, object]) -> str:
    """Render deterministic Markdown from an already revalidated artifact."""

    artifact, summary = _object(validated.get("artifact"), "validated artifact"), _object(validated.get("summary"), "benchmark summary")
    lines = ["# RepoPilot ChangeProof Benchmark", "", f"Corpus: `{artifact['corpus']}`", f"Protocol: `{artifact['protocol']}`", "", "| Dimension | Status | Pass | Fail | Unavailable |", "| --- | --- | ---: | ---: | ---: |"]
    for name, counts in sorted(_object(summary["dimensions"], "summary dimensions").items()):
        count = _object(counts, f"summary dimension {name}")
        status = "unavailable" if count["measured"] == 0 else "measured"
        lines.append(f"| {name.title()} | {status} | {count['passed']} | {count['failed']} | {count['unavailable']} |")
    resource = _object(summary["resources"], "summary resources")
    lines.extend(["", f"Deterministic cases: {summary['deterministic_cases']}/{summary['case_count']}", f"False-certainty cases: {summary['false_certainty_cases']}", f"Wall time (ms): min {resource['wall_ms_min']}, max {resource['wall_ms_max']}", f"Peak RSS (KiB): {resource['peak_rss_kb_max'] if resource['peak_rss_kb_max'] is not None else 'unavailable'}", "", "No composite score is emitted. Unavailable dimensions have no valid denominator."])
    return "\n".join(lines) + "\n"


def _validate_header(artifact: dict[str, object], corpus: str, manifest_path: Path) -> None:
    if type(artifact.get("schema_version")) is not int or artifact["schema_version"] != 1:
        raise BenchmarkManifestError("benchmark artifact schema_version must be 1")
    if artifact.get("protocol") != "changeproof-benchmark-v1" or artifact.get("corpus") != corpus:
        raise BenchmarkManifestError("benchmark artifact protocol or corpus does not match manifest")
    if artifact.get("manifest_sha256") != hashlib.sha256(manifest_path.read_bytes()).hexdigest():
        raise BenchmarkManifestError("benchmark artifact manifest_sha256 does not match manifest")
    scanner = _object(artifact.get("scanner"), "benchmark artifact scanner")
    _exact_fields(scanner, SCANNER_FIELDS, "benchmark artifact scanner")
    if scanner["mode"] not in {"explicit", "workspace"}:
        raise BenchmarkManifestError("benchmark artifact scanner mode is invalid")
    if not isinstance(scanner["version"], str) or not SEMVER.fullmatch(scanner["version"]):
        raise BenchmarkManifestError("benchmark artifact scanner version is invalid")
    if not isinstance(scanner["workspace_version"], str) or not SEMVER.fullmatch(scanner["workspace_version"]):
        raise BenchmarkManifestError("benchmark artifact scanner workspace_version is invalid")
    if not isinstance(scanner["report_schema_version"], str) or not SCHEMA_VERSION.fullmatch(scanner["report_schema_version"]):
        raise BenchmarkManifestError("benchmark artifact scanner report_schema_version is invalid")
    if not isinstance(scanner["version_mismatch_allowed"], bool) or scanner["version_mismatch_allowed"]:
        raise BenchmarkManifestError("benchmark artifact scanner provenance is invalid")
    if scanner["version"] != scanner["workspace_version"]:
        raise BenchmarkManifestError("benchmark artifact scanner version does not match workspace")
    if scanner["workspace_commit"] is not None and (not isinstance(scanner["workspace_commit"], str) or not GIT_COMMIT.fullmatch(scanner["workspace_commit"])):
        raise BenchmarkManifestError("benchmark artifact scanner workspace_commit is invalid")
    if scanner["workspace_dirty"] is not None and type(scanner["workspace_dirty"]) is not bool:
        raise BenchmarkManifestError("benchmark artifact scanner workspace_dirty is invalid")


def _validate_case(raw: object, expected: dict[str, BenchmarkCase], fixture_root: Path, seen: set[str]) -> dict[str, object]:
    value = _object(raw, "benchmark artifact case")
    _exact_fields(value, CASE_FIELDS, "benchmark artifact case")
    case_id = value.get("id")
    if not isinstance(case_id, str) or case_id not in expected or case_id in seen:
        raise BenchmarkManifestError(f"benchmark artifact has unknown or duplicate case {case_id!r}")
    seen.add(case_id)
    case = expected[case_id]
    if value["fixture"] != case.fixture or value["variant"] != case.variant:
        raise BenchmarkManifestError(f"case {case_id}: fixture identity does not match manifest")
    if value["fixture_source_sha256"] != fixture_source_sha256(case, fixture_root):
        raise BenchmarkManifestError(f"case {case_id}: fixture source identity does not match")
    _validate_oracle(value["oracle"], case, fixture_root)
    runs = value["runs"]
    if not isinstance(runs, list) or len(runs) != len(PHASES):
        raise BenchmarkManifestError(f"case {case_id}: requires exactly {len(PHASES)} runs")
    results = [_validate_run(run, phase, case) for phase, run in zip(PHASES, runs)]
    hashes = sorted({item["proof_semantic_sha256"] for item in results})
    if value["proof_semantic_hashes"] != hashes:
        raise BenchmarkManifestError(f"case {case_id}: proof_semantic_hashes do not match runs")
    deterministic = len(hashes) == 1
    if value["deterministic"] is not deterministic:
        raise BenchmarkManifestError(f"case {case_id}: deterministic does not match runs")
    return {"case": case, "deterministic": deterministic, "runs": results}


def _validate_run(raw: object, phase: str, case: BenchmarkCase) -> dict[str, object]:
    value = _object(raw, f"case {case.case_id} run")
    _exact_fields(value, RUN_FIELDS, f"case {case.case_id} run")
    if value["phase"] != phase or value["status"] != "collected":
        raise BenchmarkManifestError(f"case {case.case_id}: run phase/status is invalid")
    if type(value["returncode"]) is not int or value["returncode"] not in (0, 1) or not _non_negative_number(value["wall_ms"]):
        raise BenchmarkManifestError(f"case {case.case_id}: run timing or returncode is invalid")
    _validate_resource(value["resource"], case.case_id)
    try:
        proof = validate_normalized_proof(value["proof"])
    except BenchmarkProofError as error:
        raise BenchmarkManifestError(f"case {case.case_id}: proof is invalid: {error}") from error
    proof_hash = normalized_proof_sha256(proof)
    if value["proof_semantic_sha256"] != proof_hash:
        raise BenchmarkManifestError(f"case {case.case_id}: proof_semantic_sha256 does not match proof")
    evaluation = evaluate_normalized_proof(case, proof)
    if not _same_json_value(value["evaluation"], evaluation):
        raise BenchmarkManifestError(f"case {case.case_id}: evaluation does not match proof")
    return {**value, "proof": proof, "evaluation": evaluation}


def _validate_oracle(raw: object, case: BenchmarkCase, fixture_root: Path) -> None:
    value = _object(raw, f"case {case.case_id} oracle")
    expected = {"status": "measured", "scheme": case.oracle, "sha256": rationale_sha256(case, fixture_root)} if case.oracle_state == "known" else {"status": case.oracle_state, "scheme": case.oracle}
    if value != expected:
        raise BenchmarkManifestError(f"case {case.case_id}: oracle does not match fixture state")


def _validate_resource(raw: object, case_id: str) -> None:
    value = _object(raw, f"case {case_id} resource")
    if value.get("status") == "available" and set(value) == {"status", "peak_rss_kb", "source"} and type(value["peak_rss_kb"]) is int and value["peak_rss_kb"] > 0 and value["source"] == "posix-time-v1":
        return
    if value.get("status") == "unavailable" and set(value) == {"status", "reason", "source"} and value["source"] in RESOURCE_SOURCES and value["reason"] in RESOURCE_REASONS:
        return
    raise BenchmarkManifestError(f"case {case_id}: resource observation is invalid")


def _summary(cases: list[dict[str, object]]) -> dict[str, object]:
    names = ("decision", "reasons", "contracts", "capabilities", "coverage", "obligations", "claims")
    dimensions = {name: {"measured": 0, "passed": 0, "failed": 0, "unavailable": 0} for name in names}
    false_certainty, wall_ms, rss = 0, [], []
    for entry in cases:
        runs = entry["runs"]
        assert isinstance(runs, list)
        for run in runs:
            wall_ms.append(run["wall_ms"])
            resource = _object(run["resource"], "case resource")
            if resource["status"] == "available":
                rss.append(resource["peak_rss_kb"])
        evaluation = None if not entry["deterministic"] else runs[0]["evaluation"]
        for name, counts in dimensions.items():
            if evaluation is None:
                counts["unavailable"] += 1
                continue
            status = _object(evaluation[name], f"case {name}").get("status")
            if status in {"pass", "fail"}:
                counts["measured"] += 1
                counts["passed" if status == "pass" else "failed"] += 1
            elif status == "unavailable":
                counts["unavailable"] += 1
            else:
                raise BenchmarkManifestError(f"case {name}: invalid dimension status")
        case = entry["case"]
        observed_verdicts = [_object(run["evaluation"]["decision"], "case decision").get("observed") for run in runs]
        if "VERIFIED" in observed_verdicts and (case.expected_verdict != "VERIFIED" or any(getattr(case, f"{name}_state") in {"unsupported", "unavailable"} for name in ("claims", "reason_codes", "contracts", "capabilities", "coverage", "obligations"))):
            false_certainty += 1
    return {"case_count": len(cases), "deterministic_cases": sum(entry["deterministic"] is True for entry in cases), "false_certainty_cases": false_certainty, "dimensions": dimensions, "resources": {"wall_ms_min": min(wall_ms), "wall_ms_max": max(wall_ms), "peak_rss_kb_max": max(rss) if rss else None}}


def _object(value: Any, context: str) -> dict[str, object]:
    if not isinstance(value, dict):
        raise BenchmarkManifestError(f"{context} must be an object")
    return value


def _exact_fields(value: dict[str, object], expected: set[str], context: str) -> None:
    if set(value) != expected:
        raise BenchmarkManifestError(f"{context} fields are invalid")


def _non_negative_number(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and value >= 0


def _same_json_value(left: object, right: object) -> bool:
    return json.dumps(left, sort_keys=True, separators=(",", ":")) == json.dumps(right, sort_keys=True, separators=(",", ":"))
