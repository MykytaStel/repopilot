"""Collect immutable real-history benchmark observations without labels."""

from __future__ import annotations

import hashlib
import json
import subprocess
import tempfile
from pathlib import Path
from typing import Any

from real_history_contract import HoldoutCase, HoldoutManifestError
from real_history_contracts import ContractEvidenceError, contract_evidence_hash, observed_contract_ids
from zoo_scanner import ScannerPreparationError, ScannerProvenanceError, prepare_scanner


BASELINE_COMMANDS: dict[str, tuple[str, ...]] = {
    "python.compile": ("python3", "-m", "compileall", "-q", "."),
    "python.tests": ("python3", "-m", "pytest", "-q"),
    "python.lint": ("python3", "-m", "ruff", "check", "."),
    "python.typecheck": ("python3", "-m", "mypy", "."),
}


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def run_command(command: tuple[str, ...], cwd: Path, timeout_seconds: int) -> dict[str, Any]:
    try:
        process = subprocess.run(
            list(command), cwd=cwd, capture_output=True, check=False, timeout=timeout_seconds
        )
    except FileNotFoundError:
        return {"status": "unavailable", "returncode": None}
    except subprocess.TimeoutExpired as error:
        stdout = error.stdout or b""
        stderr = error.stderr or b""
        return {
            "status": "timeout",
            "returncode": None,
            "stdout_sha256": sha256_bytes(stdout if isinstance(stdout, bytes) else stdout.encode()),
            "stderr_sha256": sha256_bytes(stderr if isinstance(stderr, bytes) else stderr.encode()),
        }
    return {
        "status": "passed" if process.returncode == 0 else "failed",
        "returncode": process.returncode,
        "stdout_sha256": sha256_bytes(process.stdout),
        "stderr_sha256": sha256_bytes(process.stderr),
    }


def summarize_review(report: dict[str, Any], raw_output: bytes) -> dict[str, Any]:
    findings = report.get("findings")
    if not isinstance(findings, list):
        raise HoldoutManifestError("review report has no findings array")
    in_diff = [
        finding for finding in findings
        if isinstance(finding, dict) and finding.get("in_diff") is True
    ]
    try:
        contract_ids = observed_contract_ids(report)
    except ContractEvidenceError as error:
        raise HoldoutManifestError(f"review report has invalid contract evidence: {error}") from error
    evidence = {
        "schema_version": report.get("schema_version"),
        "repopilot_version": report.get("repopilot_version"),
        "in_diff_findings": len(in_diff),
        "out_of_diff_findings": len(findings) - len(in_diff),
        "in_diff_rule_ids": sorted(
            finding["rule_id"] for finding in in_diff if isinstance(finding.get("rule_id"), str)
        ),
        "contract_delta_ids": list(contract_ids),
        "contract_delta_count": len(contract_ids),
    }
    return {
        "report_sha256": sha256_bytes(raw_output),
        "stable_evidence_sha256": sha256_bytes(
            json.dumps(evidence, sort_keys=True, separators=(",", ":")).encode()
        ),
        **evidence,
        "contract_evidence_sha256": contract_evidence_hash(contract_ids),
    }


def empty_contract_observation() -> dict[str, Any]:
    """Return a deterministic no-delta observation for a timed-out review."""
    return {
        "contract_delta_ids": [],
        "contract_delta_count": 0,
        "contract_evidence_sha256": contract_evidence_hash(()),
    }


def clone_case(case: HoldoutCase, root: Path) -> tuple[Path, Path, Path]:
    root.mkdir(parents=True, exist_ok=True)
    repository = root / "repository"
    clone = subprocess.run(
        ["git", "clone", "--no-tags", "--filter=blob:none", "--no-checkout", case.url, str(repository)],
        capture_output=True,
        check=False,
        text=True,
    )
    if clone.returncode != 0:
        raise HoldoutManifestError(f"clone failed for {case.case_id}: {clone.stderr.strip()}")
    fetch = subprocess.run(
        ["git", "fetch", "--no-tags", "origin", case.base_sha, case.head_sha, case.merge_sha],
        cwd=repository,
        capture_output=True,
        check=False,
        text=True,
    )
    if fetch.returncode != 0:
        raise HoldoutManifestError(f"fetch failed for {case.case_id}: {fetch.stderr.strip()}")
    paths = (root / "base", root / "head", root / "merge")
    for path, revision in zip(paths, (case.base_sha, case.head_sha, case.merge_sha)):
        checkout = subprocess.run(
            ["git", "worktree", "add", "--detach", str(path), revision],
            cwd=repository,
            capture_output=True,
            check=False,
            text=True,
        )
        if checkout.returncode != 0:
            raise HoldoutManifestError(
                f"worktree failed for {case.case_id} ({revision}): {checkout.stderr.strip()}"
            )
    return paths


def collect_case(scanner: Any, case: HoldoutCase, root: Path, timeout_seconds: int) -> dict[str, Any]:
    base, head, merge = clone_case(case, root)
    baselines: dict[str, Any] = {}
    for baseline_id in case.baseline_ids:
        command = BASELINE_COMMANDS[baseline_id]
        result = run_command(command, merge, timeout_seconds)
        result["command"] = list(command)
        baselines[baseline_id] = result
    review_command = (
        *scanner.command,
        "review",
        str(head),
        "--base",
        case.base_sha,
        "--head",
        case.head_sha,
        "--format",
        "json",
        "--profile",
        "default",
        "--no-progress",
    )
    try:
        review = subprocess.run(
            list(review_command), cwd=head, capture_output=True, check=False, timeout=timeout_seconds
        )
    except subprocess.TimeoutExpired:
        review_result: dict[str, Any] = {
            "status": "timeout",
            "returncode": None,
            **empty_contract_observation(),
        }
    else:
        if review.returncode not in (0, 1):
            raise HoldoutManifestError(
                f"review failed for {case.case_id}: {review.stderr.decode(errors='replace').strip()}"
            )
        try:
            report = json.loads(review.stdout)
        except json.JSONDecodeError as error:
            raise HoldoutManifestError(f"review returned invalid JSON for {case.case_id}: {error}") from error
        review_result = summarize_review(report, review.stdout)
        review_result["status"] = "collected"
        review_result["returncode"] = review.returncode
    return {
        "id": case.case_id,
        "repo": case.repo,
        "pull_request": case.pull_request,
        "base_sha": case.base_sha,
        "head_sha": case.head_sha,
        "merge_sha": case.merge_sha,
        "label_state": case.label_state,
        "baselines": baselines,
        "review": review_result,
    }


def collect_holdout(
    repo_root: Path,
    manifest_path: Path,
    corpus: str,
    protocol: str,
    cases: list[HoldoutCase],
    scanner_path: str | None,
    allow_version_mismatch: bool,
    timeout_seconds: int,
) -> dict[str, Any]:
    try:
        scanner = prepare_scanner(repo_root, scanner_path, allow_version_mismatch)
    except (ScannerPreparationError, ScannerProvenanceError) as error:
        raise HoldoutManifestError(f"scanner preparation failed: {error}") from error
    observations: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="repopilot-real-history-") as temporary:
        root = Path(temporary)
        for index, case in enumerate(cases):
            observations.append(collect_case(scanner, case, root / str(index), timeout_seconds))
    return {
        "schema_version": 2,
        "corpus": corpus,
        "protocol": protocol,
        "manifest_sha256": sha256_bytes(manifest_path.read_bytes()),
        "scanner": {
            "mode": scanner.mode,
            "version": scanner.version,
            "report_schema_version": scanner.report_schema_version,
            "workspace_version": scanner.workspace_version,
            "workspace_commit": scanner.workspace_commit,
            "workspace_dirty": scanner.workspace_dirty,
            "version_mismatch_allowed": scanner.version_mismatch_allowed,
        },
        "cases": observations,
        "label_state": "pending" if any(case.label_state == "pending" for case in cases) else "ready",
    }
