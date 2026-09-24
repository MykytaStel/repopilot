"""Confined repeated-review collector for ChangeProof Benchmark v1."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any

from changeproof_benchmark_contract import (
    BenchmarkCase,
    BenchmarkManifestError,
    fixture_directory,
    fixture_source_sha256,
    rationale_sha256,
    validate_manifest,
)
from changeproof_benchmark_proof import evaluate_case, normalize_change_proof, normalized_proof_sha256
from differential_resource import execute_timed_command
from zoo_scanner import ScannerPreparationError, ScannerProvenanceError, prepare_scanner


PHASES = ("cold", "warm", "warm")


def collect_benchmark(
    repo_root: Path,
    manifest_path: Path,
    fixture_root: Path,
    scanner: str | None,
    timeout_seconds: int,
) -> dict[str, object]:
    """Collect three bounded semantic observations for every committed case."""

    if timeout_seconds <= 0:
        raise BenchmarkManifestError("timeout_seconds must be positive")
    if not isinstance(scanner, str) or not scanner.strip():
        raise BenchmarkManifestError("--scanner is required; benchmark collection never builds or downloads a scanner")
    corpus, cases = validate_manifest(manifest_path, fixture_root)
    try:
        scanner_info = prepare_scanner(repo_root, scanner, allow_version_mismatch=False)
    except (ScannerPreparationError, ScannerProvenanceError) as error:
        raise BenchmarkManifestError(f"scanner preparation failed: {error}") from error
    observations = [collect_case(case, fixture_root, scanner_info, timeout_seconds) for case in cases]
    return {
        "schema_version": 1,
        "protocol": "changeproof-benchmark-v1",
        "corpus": corpus,
        "manifest_sha256": _sha256_file(manifest_path),
        "scanner": _scanner_provenance(scanner_info),
        "cases": observations,
    }


def collect_case(case: BenchmarkCase, fixture_root: Path, scanner: Any, timeout_seconds: int) -> dict[str, object]:
    """Create one disposable Git diff and observe it once cold and twice warm."""

    source_hash = fixture_source_sha256(case, fixture_root)
    with tempfile.TemporaryDirectory(prefix="repopilot-changeproof-") as temporary:
        repo = materialize_fixture(case, fixture_root, Path(temporary))
        runs = []
        for phase in PHASES:
            run = dict(run_review(scanner, repo, case.profile, phase, timeout_seconds, case))
            run["phase"] = phase
            runs.append(run)
    if any(run["status"] != "collected" for run in runs):
        raise BenchmarkManifestError(f"case {case.case_id}: a benchmark review did not produce a complete proof")
    hashes = sorted({run["proof_semantic_sha256"] for run in runs if run["status"] == "collected"})
    return {
        "id": case.case_id,
        "fixture": case.fixture,
        "variant": case.variant,
        "fixture_source_sha256": source_hash,
        "oracle": read_oracle(case, fixture_root),
        "runs": runs,
        "proof_semantic_hashes": hashes,
        "deterministic": len(hashes) == 1 and len(runs) == len(PHASES) and all(
            run["status"] == "collected" for run in runs
        ),
    }


def materialize_fixture(case: BenchmarkCase, fixture_root: Path, destination: Path) -> Path:
    """Copy a reviewed fixture into a temporary repository with an uncommitted diff."""

    root = fixture_directory(case, fixture_root)
    before, after = root / "before", root / "after"
    if not before.is_dir():
        raise BenchmarkManifestError(f"case {case.case_id}: fixture before directory is missing")
    destination.mkdir(parents=True, exist_ok=True)
    _copy_tree(before, destination)
    _git(destination, "init", "-q")
    _git(destination, "config", "user.email", "repopilot@example.invalid")
    _git(destination, "config", "user.name", "RepoPilot Benchmark")
    _git(destination, "add", ".")
    _git(destination, "commit", "-qm", "benchmark before")
    if after.is_dir():
        _copy_tree(after, destination)
    return destination


def run_review(
    scanner: Any,
    repo: Path,
    profile: str,
    phase: str,
    timeout_seconds: int,
    case: BenchmarkCase,
) -> dict[str, object]:
    """Run exactly the fixed review command and retain structured bounded fields."""

    command = (*scanner.command, "review", ".", "--format", "json", "--profile", profile, "--no-progress")
    timed = execute_timed_command(command, repo, timeout_seconds)
    if timed["status"] == "timeout":
        return {"phase": phase, "status": "timeout", "wall_ms": timed["wall_ms"], "resource": timed["resource"]}
    if timed["returncode"] not in (0, 1):
        raise BenchmarkManifestError(f"case {case.case_id}: review exited with {timed['returncode']}")
    try:
        report = json.loads(timed["stdout"])
    except (TypeError, json.JSONDecodeError) as error:
        raise BenchmarkManifestError(f"case {case.case_id}: review did not emit JSON") from error
    if not isinstance(report, dict):
        raise BenchmarkManifestError(f"case {case.case_id}: review JSON root must be an object")
    proof = report.get("change_proof")
    normalized_proof = normalize_change_proof(proof)
    return {
        "phase": phase,
        "status": "collected",
        "returncode": timed["returncode"],
        "wall_ms": timed["wall_ms"],
        "resource": timed["resource"],
        "proof": normalized_proof,
        "proof_semantic_sha256": normalized_proof_sha256(normalized_proof),
        "evaluation": evaluate_case(case, report),
    }


def read_oracle(case: BenchmarkCase, fixture_root: Path) -> dict[str, object]:
    if case.oracle_state == "known":
        return {"status": "measured", "scheme": case.oracle, "sha256": rationale_sha256(case, fixture_root)}
    return {"status": case.oracle_state, "scheme": case.oracle}


def _copy_tree(source: Path, destination: Path) -> None:
    for path in sorted(source.rglob("*")):
        relative = path.relative_to(source)
        target = destination / relative
        if path.is_symlink():
            raise BenchmarkManifestError(f"fixture materialization refuses symlink: {relative}")
        if path.is_dir():
            target.mkdir(parents=True, exist_ok=True)
        elif path.is_file():
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target)
        else:
            raise BenchmarkManifestError(f"fixture materialization refuses non-file: {relative}")


def _git(root: Path, *args: str) -> None:
    process = subprocess.run(["git", *args], cwd=root, capture_output=True, check=False)
    if process.returncode != 0:
        raise BenchmarkManifestError(f"fixture git command failed: {' '.join(args)}")


def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _scanner_provenance(scanner: Any) -> dict[str, object]:
    return {
        "mode": scanner.mode,
        "version": scanner.version,
        "report_schema_version": scanner.report_schema_version,
        "workspace_version": scanner.workspace_version,
        "workspace_commit": scanner.workspace_commit,
        "workspace_dirty": scanner.workspace_dirty,
        "version_mismatch_allowed": scanner.version_mismatch_allowed,
    }
