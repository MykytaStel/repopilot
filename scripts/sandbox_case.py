"""Helpers for sandbox case preparation, evidence and normalization."""

from __future__ import annotations

import hashlib
import json
import re
import shutil
import subprocess
import uuid
from pathlib import Path
from typing import Any

from sandbox_contract import (
    PROTOCOL,
    SandboxCase,
    SandboxManifest,
    SandboxManifestError,
)
from sandbox_process import CommandResult, run_command, sha256_bytes


def _redact_command(command: tuple[str, ...] | list[str]) -> list[str]:
    sensitive = re.compile(
        r"(?i)(secret|token|password|authorization|api[_-]?key|private[_-]?key)"
    )
    redacted: list[str] = []
    redact_next = False
    for item in command:
        value = str(item)
        if redact_next or value in {"-c", "-e", "--eval"} or sensitive.search(value):
            redacted.append("[REDACTED]")
            redact_next = value in {"-c", "-e", "--eval"}
        else:
            redacted.append(value)
    return redacted


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _git_sha(path: Path) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=path,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise SandboxManifestError(
            f"cannot read source SHA: {result.stderr.strip() or 'git failed'}"
        )
    return result.stdout.strip()


def _clone_pinned(
    url: str, sha: str, destination: Path, timeout_seconds: int
) -> CommandResult:
    destination.parent.mkdir(parents=True, exist_ok=True)
    clone = run_command(
        (
            "git",
            "clone",
            "--no-tags",
            "--filter=blob:none",
            "--no-checkout",
            url,
            str(destination),
        ),
        destination.parent,
        timeout_seconds,
        "dependency clone",
    )
    if clone.status != "passed":
        return clone
    fetched = run_command(
        ("git", "fetch", "--no-tags", "origin", sha),
        destination,
        timeout_seconds,
        "pinned fetch",
    )
    if fetched.status != "passed":
        return fetched
    return run_command(
        ("git", "checkout", "--detach", sha),
        destination,
        timeout_seconds,
        "pinned checkout",
    )


def _copy_source(source: Path, destination: Path) -> None:
    source_root = source.resolve()
    for path in source_root.rglob("*"):
        if path.is_symlink():
            try:
                path.resolve(strict=False).relative_to(source_root)
            except ValueError as error:
                raise SandboxManifestError(
                    f"source symlink escapes checkout: {path.name}"
                ) from error
    shutil.copytree(source, destination, symlinks=True)


def _safe_patch_path(manifest: SandboxManifest, case: SandboxCase) -> Path | None:
    if case.patch is None:
        return None
    candidate = (manifest.path.parent / case.patch).resolve()
    try:
        candidate.relative_to(manifest.path.parent.resolve())
    except ValueError as error:
        raise SandboxManifestError("patch path escapes manifest directory") from error
    if not candidate.is_file():
        raise SandboxManifestError(f"patch does not exist: {case.patch}")
    return candidate


def _apply_patch(patch: Path, checkout: Path, reverse: bool = False) -> CommandResult:
    args = ["git", "apply", "--check"]
    if reverse:
        args.append("--reverse")
    args.append(str(patch))
    checked = run_command(tuple(args), checkout, 30, "patch check")
    if checked.status != "passed":
        return checked
    args = ["git", "apply"]
    if reverse:
        args.append("--reverse")
    args.append(str(patch))
    return run_command(tuple(args), checkout, 30, "patch apply")


def _phase(
    name: str, status: str, reason: str | None = None, **details: Any
) -> dict[str, Any]:
    phase: dict[str, Any] = {"name": name, "status": status}
    if reason:
        phase["reason"] = reason
    phase.update(details)
    return phase


def _write_artifact(output: Path, artifact: dict[str, Any]) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_name(f".{output.name}.{uuid.uuid4().hex}.tmp")
    temporary.write_text(
        json.dumps(artifact, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    temporary.replace(output)


def _base_artifact(
    manifest: SandboxManifest, case: SandboxCase, run_id: str
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "protocol": PROTOCOL,
        "corpus": manifest.corpus,
        "run_id": run_id,
        "case_id": case.case_id,
        "project_id": case.project_id,
        "manifest_sha256": manifest.sha256,
        "inputs": {
            "project_sha": manifest.project(case.project_id).sha,
            "image": case.image,
            "patch_sha256": None,
            "source_sha": None,
        },
        "policy": {
            "policy_id": manifest.policy.policy_id,
            "cpus": manifest.policy.cpus,
            "memory_mb": manifest.policy.memory_mb,
            "measured_timeout_seconds": manifest.policy.measured_timeout_seconds,
            "prepare_timeout_seconds": manifest.policy.prepare_timeout_seconds,
            "network": manifest.policy.network,
        },
        "provenance": {"scanner_command": None, "scanner_sha256": None},
        "phases": [],
        "status": "failed",
        "reason": "sandbox run did not finalize",
        "cleanup": {"status": "complete", "resources": [], "receipt": "finalized"},
    }


def _normalize_findings(payload: bytes | None, checkout: Path) -> dict[str, Any]:
    if not payload:
        return {
            "status": "unavailable",
            "reason": "scanner emitted no bounded JSON payload",
        }
    try:
        report = json.loads(payload)
    except (UnicodeDecodeError, json.JSONDecodeError):
        return {
            "status": "unavailable",
            "reason": "scanner output was truncated or invalid JSON",
        }
    findings = report.get("findings") if isinstance(report, dict) else None
    if not isinstance(findings, list):
        return {"status": "unavailable", "reason": "scanner JSON has no findings array"}
    normalized: list[dict[str, Any]] = []
    for finding in findings:
        if not isinstance(finding, dict):
            continue
        path = finding.get("path")
        if isinstance(path, str):
            try:
                raw_path = Path(path)
                candidate = raw_path if raw_path.is_absolute() else checkout / raw_path
                path = str(candidate.resolve().relative_to(checkout.resolve()))
            except ValueError:
                path = "<outside-checkout>"
        normalized.append(
            {
                "rule_id": finding.get("rule_id")
                if isinstance(finding.get("rule_id"), str)
                else "unknown",
                "path": path if isinstance(path, str) else None,
                "line": finding.get("line")
                if isinstance(finding.get("line"), int)
                else None,
                "in_diff": finding.get("in_diff")
                if isinstance(finding.get("in_diff"), bool)
                else None,
            }
        )
    normalized.sort(
        key=lambda item: (item["path"] or "", item["line"] or 0, item["rule_id"])
    )
    canonical = json.dumps(normalized, sort_keys=True, separators=(",", ":")).encode()
    return {
        "status": "measured",
        "count": len(normalized),
        "findings": normalized,
        "sha256": sha256_bytes(canonical),
    }
