"""Collect repeated baseline and RepoPilot observations for utility benchmarking."""

from __future__ import annotations

import hashlib
import json
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any

from differential_contract import validate_differential
from differential_evidence import normalize_baseline_evidence, normalize_review_verification
from differential_identity import review_comparable_diagnostic_keys
from differential_manifest import load_differential
from differential_novelty import evidence_keys, novel_evidence_keys
from differential_resource import execute_timed_command
from differential_telemetry import build_command_telemetry, build_review_telemetry
from real_history_contract import HoldoutCase, HoldoutManifestError, validate_manifest
from real_history_runner import BASELINE_COMMANDS, clone_case, summarize_review
from zoo_scanner import ScannerPreparationError, ScannerProvenanceError, prepare_scanner

DIFFERENTIAL_ARTIFACT_SCHEMA_VERSION = 2


def _resource_phase(repeat: int) -> str:
    return "cold" if repeat == 1 else "warm"


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def run_timed_command(
    command: tuple[str, ...], cwd: Path, timeout_seconds: int, baseline_id: str | None = None
) -> dict[str, Any]:
    observation = execute_timed_command(command, cwd, timeout_seconds)
    status = observation["status"]
    returncode = observation["returncode"]
    stdout = observation["stdout"]
    stderr = observation["stderr"]
    resource = observation["resource"]
    result: dict[str, Any] = {
        "status": status,
        "returncode": returncode,
        "command": list(command),
        "wall_ms": observation["wall_ms"],
        "stdout_sha256": sha256_bytes(stdout if isinstance(stdout, bytes) else stdout.encode()),
        "stderr_sha256": sha256_bytes(stderr if isinstance(stderr, bytes) else stderr.encode()),
        "resource_status": resource["status"],
        "resource_source": resource["source"],
    }
    if resource["status"] == "available":
        result["child_max_rss_kb"] = resource["peak_rss_kb"]
    else:
        result["resource_reason"] = resource["reason"]
    result["telemetry"] = build_command_telemetry(result["wall_ms"])
    result["evidence"] = (
        normalize_baseline_evidence(baseline_id, stdout, stderr, returncode, cwd)
        if baseline_id is not None and status in {"passed", "failed"}
        else {"status": "unavailable", "reason": "baseline command did not complete"}
    )
    return result


def _scan_base(scanner: Any, case: HoldoutCase, base: Path, timeout_seconds: int) -> dict[str, Any]:
    command = (*scanner.command, "scan", str(base), "--format", "json", "--profile", "default", "--no-progress")
    started = time.perf_counter()
    try:
        process = subprocess.run(list(command), cwd=base, capture_output=True, check=False, timeout=timeout_seconds)
    except subprocess.TimeoutExpired:
        wall_ms = round((time.perf_counter() - started) * 1000, 3)
        return {
            "status": "timeout",
            "returncode": None,
            "wall_ms": wall_ms,
            "telemetry": build_command_telemetry(wall_ms),
        }
    if process.returncode not in (0, 1):
        raise HoldoutManifestError(
            f"differential base scan failed for {case.case_id}: {process.stderr.decode(errors='replace').strip()}"
        )
    try:
        report = json.loads(process.stdout)
    except json.JSONDecodeError as error:
        raise HoldoutManifestError(
            f"differential base scan returned invalid JSON for {case.case_id}: {error}"
        ) from error
    findings = report.get("findings")
    if not isinstance(findings, list):
        raise HoldoutManifestError(f"differential base scan has no findings array for {case.case_id}")
    wall_ms = round((time.perf_counter() - started) * 1000, 3)
    return {
        "status": "collected",
        "returncode": process.returncode,
        "wall_ms": wall_ms,
        "command": list(command),
        "evidence_keys": sorted(evidence_keys(findings)),
        "telemetry": build_command_telemetry(wall_ms),
    }


def _review_command(
    scanner: Any,
    base_sha: str,
    head_sha: str,
    head: Path,
    review_config: Path | None,
    review_verify: tuple[str, ...],
) -> tuple[str, ...]:
    scanner_command = scanner.command if hasattr(scanner, "command") else scanner
    command = (
        *scanner_command,
        "review",
        str(head),
        "--base",
        base_sha,
        "--head",
        head_sha,
        "--format",
        "json",
        "--profile",
        "default",
        "--no-progress",
    )
    if review_config is not None:
        command += ("--config", str(review_config))
    for check_id in review_verify:
        command += ("--verify", check_id)
    return command


def _review_once(
    scanner: Any,
    case: HoldoutCase,
    head: Path,
    base_evidence: set[str],
    timeout_seconds: int,
    review_config: Path | None = None,
    review_verify: tuple[str, ...] = (),
) -> dict[str, Any]:
    command = _review_command(
        scanner,
        case.base_sha,
        case.head_sha,
        head,
        review_config,
        review_verify,
    )
    observation = execute_timed_command(command, head, timeout_seconds)
    if observation["status"] == "timeout":
        resource = observation["resource"]
        result = {
            "status": "timeout",
            "returncode": None,
            "wall_ms": observation["wall_ms"],
            "telemetry": build_command_telemetry(observation["wall_ms"]),
            "resource_status": resource["status"],
            "resource_source": resource["source"],
            "in_diff_evidence_keys": [],
            "novel_in_diff_evidence_keys": [],
            "in_diff_comparison_keys": [],
            "verification_comparison": normalize_review_verification({}, head),
        }
        if resource["status"] == "unavailable":
            result["resource_reason"] = resource["reason"]
        return result
    if observation["status"] == "unavailable":
        raise HoldoutManifestError(f"differential review command was unavailable for {case.case_id}")
    if observation["returncode"] not in (0, 1):
        raise HoldoutManifestError(
            f"differential review failed for {case.case_id}: "
            f"{observation['stderr'].decode(errors='replace').strip()}"
        )
    try:
        report = json.loads(observation["stdout"])
    except json.JSONDecodeError as error:
        raise HoldoutManifestError(f"differential review returned invalid JSON for {case.case_id}: {error}") from error
    result = _build_review_result(
        report,
        observation["stdout"],
        observation["returncode"],
        base_evidence,
        observation["wall_ms"],
        head,
    )
    resource = observation["resource"]
    result["resource_status"] = resource["status"]
    result["resource_source"] = resource["source"]
    if resource["status"] == "available":
        result["child_max_rss_kb"] = resource["peak_rss_kb"]
    else:
        result["resource_reason"] = resource["reason"]
    return result


def _build_review_result(
    report: dict[str, Any],
    raw_output: bytes,
    returncode: int,
    base_evidence: set[str],
    wall_ms: float,
    cwd: Path,
) -> dict[str, Any]:
    in_diff_findings = [
        finding for finding in report["findings"] if isinstance(finding, dict) and finding.get("in_diff") is True
    ]
    in_diff_keys = sorted(evidence_keys(in_diff_findings))
    novel_keys = novel_evidence_keys(in_diff_findings, base_evidence)
    comparison_keys = review_comparable_diagnostic_keys(report)
    verification_comparison = normalize_review_verification(report, cwd)
    result = {
        **summarize_review(report, raw_output),
        "status": "collected",
        "returncode": returncode,
        "wall_ms": wall_ms,
        "in_diff_evidence_keys": in_diff_keys,
        "novel_in_diff_evidence_keys": novel_keys,
        "in_diff_comparison_keys": comparison_keys,
        "verification_comparison": verification_comparison,
    }
    result["telemetry"] = build_review_telemetry(report, wall_ms, bool(novel_keys))
    return result


def collect_case(
    scanner: Any,
    case: HoldoutCase,
    baseline_ids: tuple[str, ...],
    repetitions: int,
    root: Path,
    timeout_seconds: int,
    review_config: Path | None = None,
    review_verify: tuple[str, ...] = (),
) -> dict[str, Any]:
    base, head, merge = clone_case(case, root)
    base_scan = _scan_base(scanner, case, base, timeout_seconds)
    base_evidence = set(base_scan.get("evidence_keys", []))
    baselines = {baseline_id: [] for baseline_id in baseline_ids}
    reviews = []
    for repeat in range(1, repetitions + 1):
        for baseline_id in baseline_ids:
            result = run_timed_command(BASELINE_COMMANDS[baseline_id], merge, timeout_seconds, baseline_id)
            result["repeat"] = repeat
            result["resource_phase"] = _resource_phase(repeat)
            baselines[baseline_id].append(result)
        review = _review_once(
            scanner,
            case,
            head,
            base_evidence,
            timeout_seconds,
            review_config,
            review_verify,
        )
        review["repeat"] = repeat
        review["resource_phase"] = _resource_phase(repeat)
        reviews.append(review)
    stable_hashes = [run["stable_evidence_sha256"] for run in reviews if run["status"] == "collected"]
    return {
        "id": case.case_id,
        "repo": case.repo,
        "pull_request": case.pull_request,
        "base_sha": case.base_sha,
        "head_sha": case.head_sha,
        "merge_sha": case.merge_sha,
        "base_scan": base_scan,
        "baselines": baselines,
        "reviews": reviews,
        "determinism": {
            "comparable_runs": len(stable_hashes),
            "stable_evidence_unique": sorted(set(stable_hashes)),
            "stable_evidence_deterministic": len(stable_hashes) > 1 and len(set(stable_hashes)) == 1,
        },
    }


def collect_differential(
    repo_root: Path,
    manifest_path: Path,
    differential_path: Path,
    rules_reference: Path,
    zoo_manifest: Path,
    scanner_path: str | None,
    allow_version_mismatch: bool,
    timeout_seconds: int,
    review_config: Path | None = None,
    review_verify: tuple[str, ...] = (),
) -> dict[str, Any]:
    summary = validate_differential(differential_path, manifest_path, rules_reference, zoo_manifest)
    corpus, _, holdout_cases = validate_manifest(manifest_path, rules_reference, zoo_manifest)
    _, _, _, _, differential_cases = load_differential(differential_path)
    case_config = {case.case_id: case for case in differential_cases}
    try:
        scanner = prepare_scanner(repo_root, scanner_path, allow_version_mismatch)
    except (ScannerPreparationError, ScannerProvenanceError) as error:
        raise HoldoutManifestError(f"scanner preparation failed: {error}") from error
    observations = []
    with tempfile.TemporaryDirectory(prefix="repopilot-differential-") as temporary:
        for index, case in enumerate(holdout_cases):
            config = case_config[case.case_id]
            observations.append(
                collect_case(
                    scanner,
                    case,
                    config.baseline_ids,
                    summary["repetitions"],
                    Path(temporary) / str(index),
                    timeout_seconds,
                    review_config,
                    review_verify,
                )
            )
    return {
        "schema_version": DIFFERENTIAL_ARTIFACT_SCHEMA_VERSION,
        "corpus": corpus,
        "protocol": summary["protocol"],
        "manifest_sha256": hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
        "differential_manifest_sha256": hashlib.sha256(differential_path.read_bytes()).hexdigest(),
        "scanner": {
            "mode": scanner.mode,
            "version": scanner.version,
            "report_schema_version": scanner.report_schema_version,
            "workspace_version": scanner.workspace_version,
            "workspace_commit": scanner.workspace_commit,
            "workspace_dirty": scanner.workspace_dirty,
            "version_mismatch_allowed": scanner.version_mismatch_allowed,
        },
        "review_verification": {
            "checks": list(review_verify),
            "config_sha256": (
                hashlib.sha256(review_config.read_bytes()).hexdigest()
                if review_config is not None
                else None
            ),
        },
        "repetitions": summary["repetitions"],
        "cases": observations,
        "label_state": "pending",
        "limitation": "observations only; no labels, scoring, or utility claim",
    }
