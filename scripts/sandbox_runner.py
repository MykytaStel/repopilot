"""Safe, artifact-first orchestration for one validation sandbox case."""

from __future__ import annotations

import tempfile
import uuid
from pathlib import Path
from typing import Any

from sandbox_contract import PROTOCOL, SandboxManifestError, load_manifest
from sandbox_process import (
    CommandResult,
    DockerAdapter,
    DockerResult,
    SubprocessDockerAdapter,
    run_command,
    sha256_bytes,
)
from sandbox_case import (
    _apply_patch,
    _base_artifact,
    _copy_source,
    _git_sha,
    _normalize_findings,
    _phase,
    _redact_command,
    _safe_patch_path,
    _clone_pinned,
    sha256_file,
    _write_artifact,
)


MAX_SCANNER_REPORT_BYTES = 8 * 1024 * 1024


def _scanner_report_payload(
    report_path: Path, fallback: bytes | None, fallback_truncated: bool
) -> tuple[bytes | None, str | None]:
    if report_path.is_file():
        try:
            size = report_path.stat().st_size
            if size > MAX_SCANNER_REPORT_BYTES:
                return None, "scanner report exceeds the bounded report size"
            return report_path.read_bytes(), None
        except OSError:
            return None, "scanner report could not be read"
    if fallback is not None and not fallback_truncated:
        return fallback, None
    return None, "scanner did not produce a complete JSON report"


def _run_static_analysis(
    scanner_command: list[str],
    checkout: Path,
    report_path: Path,
    analysis_options: tuple[str, ...],
    manifest: Any,
    label: str,
    profile: str,
) -> tuple[CommandResult, dict[str, Any], dict[str, Any]]:
    analysis = run_command(
        tuple(
            (
                *scanner_command,
                "scan",
                str(checkout),
                *analysis_options,
                "--format",
                "json",
                "--output",
                str(report_path),
                "--profile",
                profile,
                "--no-progress",
            )
        ),
        checkout,
        manifest.policy.measured_timeout_seconds,
        label,
        manifest.policy.log_limit_bytes,
        True,
    )
    payload, report_reason = _scanner_report_payload(
        report_path, analysis.payload, analysis.stdout_truncated
    )
    normalized = (
        _normalize_findings(payload, checkout)
        if payload is not None
        else {"status": "unavailable", "reason": report_reason}
    )
    details = analysis.as_dict()
    if report_path.is_file():
        details["report_sha256"] = sha256_file(report_path)
        details["report_bytes"] = report_path.stat().st_size
    details["normalized_findings"] = normalized
    return analysis, normalized, details


def run_case(
    manifest_path: Path,
    case_id: str,
    output: Path,
    *,
    source_override: Path | None = None,
    scanner: tuple[str, ...] | None = None,
    dry_run: bool = False,
    docker: DockerAdapter | None = None,
    work_root: Path | None = None,
) -> dict[str, Any]:
    run_id = uuid.uuid4().hex
    try:
        manifest = load_manifest(manifest_path)
        case = manifest.case(case_id)
        artifact = _base_artifact(manifest, case, run_id)
    except Exception as error:
        artifact = {
            "schema_version": 1,
            "protocol": PROTOCOL,
            "run_id": run_id,
            "status": "failed",
            "reason": str(error),
            "phases": [],
            "cleanup": {
                "status": "complete",
                "resources": [],
                "receipt": "manifest-finalization",
            },
        }
        _write_artifact(output, artifact)
        return artifact
    case_root: Path | None = None
    try:
        patch = _safe_patch_path(manifest, case)
        artifact["inputs"]["patch_sha256"] = sha256_file(patch) if patch else None
        if dry_run:
            if source_override is not None:
                source = source_override.resolve()
                if not source.is_dir():
                    raise SandboxManifestError(
                        f"source override is not a directory: {source}"
                    )
                actual_sha = _git_sha(source)
                artifact["inputs"]["source_sha"] = actual_sha
                if actual_sha != manifest.project(case.project_id).sha:
                    raise SandboxManifestError(
                        f"source SHA {actual_sha} does not match pinned project SHA"
                    )
            artifact["phases"] = [
                _phase(name, "dry-run")
                for name in (
                    "prepare",
                    "baseline",
                    "mutate",
                    "analyze",
                    "oracle",
                    "revert",
                    "collect",
                )
            ]
            artifact["status"], artifact["reason"] = (
                "dry-run",
                "manifest, pin, patch and policy validated",
            )
            _write_artifact(output, artifact)
            return artifact
        root = (work_root or output.parent / "work").resolve()
        root.mkdir(parents=True, exist_ok=True)
        case_root = Path(tempfile.mkdtemp(prefix=f"{case.case_id}-{run_id}-", dir=root))
        if source_override is not None:
            source = source_override.resolve()
            if not source.is_dir():
                raise SandboxManifestError(
                    f"source override is not a directory: {source}"
                )
            actual_sha = _git_sha(source)
            artifact["inputs"]["source_sha"] = actual_sha
            if actual_sha != manifest.project(case.project_id).sha:
                raise SandboxManifestError(
                    f"source SHA {actual_sha} does not match pinned project SHA"
                )
        else:
            source = case_root / "pinned-source"
            clone = _clone_pinned(
                manifest.project(case.project_id).url,
                manifest.project(case.project_id).sha,
                source,
                manifest.policy.prepare_timeout_seconds,
            )
            if clone.status != "passed":
                artifact["phases"].append(
                    _phase(
                        "prepare", clone.status, clone.reason, result=clone.as_dict()
                    )
                )
                raise SandboxManifestError(f"pinned source preparation {clone.status}")
            artifact["inputs"]["source_sha"] = _git_sha(source)
        before, after = case_root / "before", case_root / "after"
        _copy_source(source, before)
        artifact["phases"].append(
            _phase("prepare", "passed", source_sha=artifact["inputs"]["source_sha"])
        )
        docker_adapter = docker or SubprocessDockerAdapter()
        baseline = (
            docker_adapter.run(
                case.setup,
                before,
                case.image,
                manifest.policy.prepare_timeout_seconds,
                manifest.policy,
                run_id,
            )
            if case.setup
            else DockerResult("passed", 0, sha256_bytes(b""), sha256_bytes(b""), 0.0)
        )
        artifact["phases"].append(
            _phase(
                "baseline",
                baseline.status,
                baseline.reason,
                command=_redact_command(case.setup),
                result=baseline.as_dict(),
            )
        )
        if baseline.status not in {"passed", "unavailable"}:
            raise SandboxManifestError(f"baseline oracle {baseline.status}")
        _copy_source(before, after)
        mutation = (
            _apply_patch(patch, after)
            if patch
            else CommandResult(
                "passed",
                0,
                sha256_bytes(b""),
                sha256_bytes(b""),
                0.0,
                "no mutation patch",
            )
        )
        artifact["phases"].append(
            _phase(
                "mutate", mutation.status, mutation.reason, result=mutation.as_dict()
            )
        )
        if mutation.status != "passed":
            raise SandboxManifestError("mutation patch could not be applied")
        report_path: Path | None = None
        baseline_analysis: CommandResult | None = None
        baseline_normalized: dict[str, Any] | None = None
        baseline_details: dict[str, Any] | None = None
        if scanner:
            scanner_path = Path(scanner[0]).expanduser()
            scanner_command = list(scanner)
            if scanner_path.is_file():
                scanner_path = scanner_path.resolve()
                scanner_command[0] = str(scanner_path)
                artifact["provenance"]["scanner_sha256"] = sha256_file(scanner_path)
            artifact["provenance"]["scanner_command"] = _redact_command(scanner_command)
            analysis_options = ("--changed",) if case.analysis_mode == "changed" else ()
            if case.expected_rule_ids:
                baseline_report_path = case_root / ".repopilot-sandbox-baseline-report.json"
                baseline_analysis, baseline_normalized, baseline_details = (
                    _run_static_analysis(
                        scanner_command,
                        before,
                        baseline_report_path,
                        analysis_options,
                        manifest,
                        "baseline static analysis",
                        case.profile,
                    )
                )
            report_path = case_root / ".repopilot-sandbox-report.json"
            analysis, normalized, analysis_result = _run_static_analysis(
                scanner_command,
                after,
                report_path,
                analysis_options,
                manifest,
                "static analysis",
                case.profile,
            )
            if baseline_details is not None:
                analysis_result["baseline_analysis"] = baseline_details
                analysis_result["baseline_normalized_findings"] = baseline_normalized
        else:
            analysis = CommandResult(
                "unavailable",
                None,
                sha256_bytes(b""),
                sha256_bytes(b""),
                0.0,
                "scanner is unavailable",
            )
            normalized = {"status": "unavailable", "reason": "scanner is unavailable"}
            analysis_result = analysis.as_dict()
            analysis_result["normalized_findings"] = normalized
        analysis_status = analysis.status
        analysis_reason = analysis.reason
        if analysis.status == "passed" and normalized.get("status") != "measured":
            analysis_status = "unavailable"
            analysis_reason = str(
                normalized.get("reason") or "normalized scanner output is unavailable"
            )
        if (
            baseline_analysis is not None
            and (
                baseline_analysis.status != "passed"
                or not baseline_normalized
                or baseline_normalized.get("status") != "measured"
            )
        ):
            analysis_status = "unavailable"
            analysis_reason = "baseline static analysis is unavailable"
        artifact["phases"].append(
            _phase("analyze", analysis_status, analysis_reason, result=analysis_result)
        )
        oracle = docker_adapter.run(
            case.oracle,
            after,
            case.image,
            manifest.policy.measured_timeout_seconds,
            manifest.policy,
            run_id,
        )
        artifact["phases"].append(
            _phase(
                "oracle",
                oracle.status,
                oracle.reason,
                command=_redact_command(case.oracle),
                result=oracle.as_dict(),
            )
        )
        revert = (
            _apply_patch(patch, after, reverse=True)
            if patch
            else CommandResult(
                "passed",
                0,
                sha256_bytes(b""),
                sha256_bytes(b""),
                0.0,
                "no mutation patch",
            )
        )
        artifact["phases"].append(
            _phase("revert", revert.status, revert.reason, result=revert.as_dict())
        )
        artifact["phases"].append(_phase("collect", "passed"))
        unavailable_phase = next(
            (
                phase
                for phase in artifact["phases"]
                if phase["status"] in {"unavailable", "timeout"}
            ),
            None,
        )
        if oracle.status != case.expected_oracle:
            artifact["status"] = oracle.status
        elif unavailable_phase is not None:
            artifact["status"] = "unavailable"
        else:
            artifact["status"] = "passed"
        artifact["reason"] = (
            f"oracle status {oracle.status}; expected {case.expected_oracle}"
        )
        if unavailable_phase is not None:
            artifact["reason"] += (
                f"; {unavailable_phase['name']} is {unavailable_phase['status']}"
            )
    except Exception as error:
        artifact["status"] = "failed"
        artifact["reason"] = str(error)
        if not artifact["phases"] or artifact["phases"][-1]["name"] != "collect":
            artifact["phases"].append(_phase("collect", "failed", str(error)))
    finally:
        artifact["cleanup"] = {
            "status": "complete",
            "resources": [],
            "receipt": f"run {run_id} finalized",
        }
        _write_artifact(output, artifact)
    return artifact
