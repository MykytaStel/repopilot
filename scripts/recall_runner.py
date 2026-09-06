"""Frozen scanner execution and metric generation for recall evidence."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

from recall_contract import RecallCase, RecallManifestError
from zoo_scanner import (
    ScannerPreparationError,
    ScannerProvenance,
    ScannerProvenanceError,
    prepare_scanner,
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def evaluate_case_report(case: RecallCase, report: dict[str, object]) -> dict[str, object]:
    findings = report.get("findings")
    if not isinstance(findings, list):
        raise RecallManifestError(f"scan report for {case.case_id} has no findings array")
    observed = sorted(
        finding.get("rule_id")
        for finding in findings
        if isinstance(finding, dict) and isinstance(finding.get("rule_id"), str)
    )
    matching = observed.count(case.rule_id)
    passed = matching > 0 if case.outcome == "must-fire" else matching == 0
    return {
        "id": case.case_id,
        "rule_id": case.rule_id,
        "profile": case.profile,
        "kind": case.kind,
        "path": case.path,
        "expected": case.outcome,
        "observed_rule_ids": observed,
        "matching_findings": matching,
        "status": "pass" if passed else "fail",
    }


def scan_case(
    scanner: ScannerProvenance,
    case_root: Path,
    case: RecallCase,
    config_path: Path | None,
) -> dict[str, object]:
    command = [*scanner.command, "scan", str(case_root), "--format", "json", "--profile", case.profile, "--no-progress"]
    if config_path is not None:
        command.extend(["--config", str(config_path)])
    proc = subprocess.run(command, text=True, capture_output=True, check=False)
    if proc.returncode not in (0, 1):
        detail = proc.stderr.strip() or proc.stdout.strip()
        raise RecallManifestError(
            f"scan failed for {case.case_id} ({proc.returncode}): {detail}"
        )
    try:
        report = json.loads(proc.stdout)
    except json.JSONDecodeError as error:
        raise RecallManifestError(f"scan produced invalid JSON for {case.case_id}: {error}") from error
    return evaluate_case_report(case, report)


def run_corpus(
    repo_root: Path,
    manifest_path: Path,
    rules_reference_path: Path,
    corpus: str,
    cases: list[RecallCase],
    scanner_path: str | None,
    allow_version_mismatch: bool,
) -> dict[str, object]:
    try:
        scanner = prepare_scanner(repo_root, scanner_path, allow_version_mismatch)
    except (ScannerPreparationError, ScannerProvenanceError) as error:
        raise RecallManifestError(f"scanner preparation failed: {error}") from error
    config_path = repo_root / "repopilot.toml"
    if not config_path.is_file():
        config_path = None
    results: list[dict[str, object]] = []
    with tempfile.TemporaryDirectory(prefix="repopilot-recall-") as temporary:
        temporary_root = Path(temporary)
        for index, case in enumerate(cases):
            source = (manifest_path.parent / case.path).resolve()
            case_root = temporary_root / str(index)
            shutil.copytree(source, case_root)
            results.append(scan_case(scanner, case_root, case, config_path))
    passed = all(result["status"] == "pass" for result in results)
    by_id = {case.case_id: case for case in cases}
    true_positives = sum(
        by_id[result["id"]].kind == "seeded-defect" and result["status"] == "pass"
        for result in results
    )
    false_negatives = sum(
        by_id[result["id"]].kind == "seeded-defect" and result["status"] == "fail"
        for result in results
    )
    true_negatives = sum(
        by_id[result["id"]].kind == "safe-guard" and result["status"] == "pass"
        for result in results
    )
    false_positives = sum(
        by_id[result["id"]].kind == "safe-guard" and result["status"] == "fail"
        for result in results
    )
    return {
        "schema_version": 1,
        "corpus": corpus,
        "manifest_sha256": sha256_file(manifest_path),
        "rules_reference_sha256": sha256_file(rules_reference_path),
        "scanner": {
            "mode": scanner.mode,
            "version": scanner.version,
            "report_schema_version": scanner.report_schema_version,
            "workspace_version": scanner.workspace_version,
            "workspace_commit": scanner.workspace_commit,
            "workspace_dirty": scanner.workspace_dirty,
            "version_mismatch_allowed": scanner.version_mismatch_allowed,
            "config_sha256": sha256_file(config_path) if config_path else None,
        },
        "cases": results,
        "counts": {
            "total": len(results),
            "passed": sum(result["status"] == "pass" for result in results),
            "failed": sum(result["status"] == "fail" for result in results),
            "seeded_defects": sum(case.kind == "seeded-defect" for case in cases),
            "safe_guards": sum(case.kind == "safe-guard" for case in cases),
        },
        "metrics": {
            "scope": "corpus-only",
            "true_positives": true_positives,
            "false_negatives": false_negatives,
            "true_negatives": true_negatives,
            "false_positives": false_positives,
            "recall": true_positives / (true_positives + false_negatives)
            if true_positives + false_negatives
            else None,
            "specificity": true_negatives / (true_negatives + false_positives)
            if true_negatives + false_positives
            else None,
        },
        "status": "pass" if passed else "fail",
    }
