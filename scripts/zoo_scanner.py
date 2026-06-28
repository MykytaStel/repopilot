"""Scanner preparation for the real-repo validation zoo."""

from __future__ import annotations

import json
import os
import re
import shlex
import subprocess
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any


WORKSPACE_PROFILE = "release"


class ScannerPreparationError(RuntimeError):
    pass

class ScannerProvenanceError(RuntimeError):
    pass

@dataclass(frozen=True)
class ReportIdentity:
    repopilot_version: str
    schema_version: str


@dataclass(frozen=True)
class ScannerProvenance:
    mode: str
    command: tuple[str, ...]
    binary: Path
    version: str
    report_schema_version: str
    workspace_version: str
    profile: str | None = None
    workspace_commit: str | None = None
    workspace_dirty: bool | None = None
    version_mismatch_allowed: bool = False

def run(cmd: list[str], cwd: Path | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=str(cwd) if cwd else None,
        text=True,
        capture_output=True,
        check=check,
    )


def format_cmd(cmd: list[str] | tuple[str, ...]) -> str:
    return shlex.join(str(part) for part in cmd)

def completed_diagnostic(proc: subprocess.CompletedProcess[str]) -> str:
    outcome = "failed" if proc.returncode else "completed"
    details = [f"command {outcome} ({proc.returncode}): {format_cmd(tuple(str(a) for a in proc.args))}"]
    if proc.stderr.strip():
        details.append(f"stderr:\n{proc.stderr.strip()}")
    if proc.stdout.strip():
        details.append(f"stdout:\n{proc.stdout.strip()}")
    return "\n".join(details)


def prepare_scanner(repo_root: Path, explicit: str | None, allow_version_mismatch: bool) -> ScannerProvenance:
    workspace_version = read_workspace_version(repo_root)
    if explicit:
        mode = "explicit"
        profile = None
        binary = resolve_explicit_binary(explicit)
    else:
        mode = "workspace"
        profile = WORKSPACE_PROFILE
        binary = build_workspace_binary(repo_root)

    command = (str(binary),)
    cli_version = read_cli_version(command)
    default_identity, strict_identity = read_report_identities(command)
    validate_report_consistency(cli_version, default_identity, strict_identity)
    mismatch_allowed = validate_workspace_version(
        mode,
        cli_version,
        workspace_version,
        allow_version_mismatch,
    )
    commit, dirty = workspace_git_state(repo_root) if mode == "workspace" else (None, None)
    return ScannerProvenance(
        mode=mode,
        command=command,
        binary=binary,
        version=cli_version,
        report_schema_version=default_identity.schema_version,
        workspace_version=workspace_version,
        profile=profile,
        workspace_commit=commit,
        workspace_dirty=dirty,
        version_mismatch_allowed=mismatch_allowed,
    )


def print_scanner_provenance(scanner: ScannerProvenance) -> None:
    print(f"scanner mode: {scanner.mode}")
    if scanner.profile:
        print(f"scanner profile: {scanner.profile}")
    print(f"scanner binary: {scanner.binary}")
    print(f"scanner version: {scanner.version}")
    print(f"scanner report schema: {scanner.report_schema_version}")
    if scanner.version_mismatch_allowed:
        print(f"workspace version: {scanner.workspace_version} (mismatch allowed)")
    if scanner.workspace_commit:
        print(f"workspace commit: {scanner.workspace_commit}")
    if scanner.workspace_dirty is not None:
        print(f"workspace dirty: {str(scanner.workspace_dirty).lower()}")
    print()


def build_workspace_binary(repo_root: Path) -> Path:
    cmd = ["cargo", "build", "--release", "--manifest-path", str(repo_root / "Cargo.toml")]
    cmd.append("--message-format=json-render-diagnostics")
    proc = run(cmd, cwd=repo_root, check=False)
    if proc.returncode != 0:
        raise ScannerPreparationError("scanner build failed\n" + completed_diagnostic(proc))
    executable = cargo_build_executable(proc.stdout)
    if executable is None:
        raise ScannerPreparationError(
            "scanner build did not report a repopilot executable\n"
            + completed_diagnostic(proc)
        )
    if not executable.exists():
        raise ScannerPreparationError(
            f"cargo reported repopilot executable that does not exist: {executable}"
        )
    return executable


def cargo_build_executable(stdout: str) -> Path | None:
    executable: Path | None = None
    for line in stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("reason") != "compiler-artifact":
            continue
        target, kind = message.get("target"), None
        if isinstance(target, dict):
            kind = target.get("kind")
        if not isinstance(target, dict) or target.get("name") != "repopilot":
            continue
        raw_executable = message.get("executable")
        if isinstance(kind, list) and "bin" in kind and isinstance(raw_executable, str):
            executable = Path(raw_executable)
    return executable


def resolve_explicit_binary(explicit: str) -> Path:
    binary = Path(explicit).expanduser()
    if not binary.is_absolute():
        binary = (Path.cwd() / binary).resolve()
    if not binary.exists():
        raise ScannerPreparationError(f"explicit repopilot binary does not exist: {binary}")
    if not os.access(binary, os.X_OK):
        raise ScannerPreparationError(f"explicit repopilot binary is not executable: {binary}")
    return binary


def read_workspace_version(repo_root: Path) -> str:
    data = tomllib.loads((repo_root / "Cargo.toml").read_text(encoding="utf-8"))
    package = data.get("package")
    if not isinstance(package, dict) or not isinstance(package.get("version"), str):
        raise ScannerPreparationError("could not read package.version from Cargo.toml")
    return package["version"]


def read_cli_version(command: tuple[str, ...]) -> str:
    proc = run([*command, "--version"], check=False)
    if proc.returncode != 0:
        raise ScannerProvenanceError("scanner version check failed\n" + completed_diagnostic(proc))
    text = "\n".join(part.strip() for part in (proc.stdout, proc.stderr) if part.strip())
    match = re.search(r"(?im)^repopilot\s+([^\s]+)\s*$", text)
    if not match:
        raise ScannerProvenanceError(
            "scanner did not identify itself as RepoPilot via --version\n"
            f"output:\n{text}"
        )
    return match.group(1)


def read_report_identities(command: tuple[str, ...]) -> tuple[ReportIdentity, ReportIdentity]:
    with tempfile.TemporaryDirectory(prefix="repopilot-zoo-identity-") as tmp:
        root = Path(tmp)
        return (
            read_report_identity(command, root, "default"),
            read_report_identity(command, root, "strict"),
        )


def read_report_identity(command: tuple[str, ...], root: Path, profile: str) -> ReportIdentity:
    proc = run(
        [*command, "scan", str(root), "--format", "json", "--profile", profile],
        check=False,
    )
    if proc.returncode not in (0, 1):
        raise ScannerProvenanceError(
            f"scanner identity scan failed for {profile} profile\n"
            + completed_diagnostic(proc)
        )
    try:
        report = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise ScannerProvenanceError(
            f"scanner identity scan produced invalid JSON for {profile} profile: {exc}"
        ) from exc
    return validate_report_identity(report, profile)


def validate_report_identity(report: dict[str, Any], profile: str) -> ReportIdentity:
    schema_version = report.get("schema_version")
    repopilot_version = report.get("repopilot_version")
    envelope = report.get("report")
    if not isinstance(schema_version, str) or not isinstance(repopilot_version, str):
        raise ScannerProvenanceError(
            f"{profile} report is missing top-level schema/version metadata"
        )
    if not isinstance(envelope, dict):
        raise ScannerProvenanceError(f"{profile} report is missing report envelope metadata")
    if envelope.get("kind") != "scan":
        raise ScannerProvenanceError(
            f"{profile} report envelope kind is not 'scan': {envelope.get('kind')!r}"
        )
    if envelope.get("schema_version") != schema_version:
        raise ScannerProvenanceError(
            f"{profile} report schema mismatch: top-level {schema_version!r}, "
            f"envelope {envelope.get('schema_version')!r}"
        )
    if envelope.get("repopilot_version") != repopilot_version:
        raise ScannerProvenanceError(
            f"{profile} report version mismatch: top-level {repopilot_version!r}, "
            f"envelope {envelope.get('repopilot_version')!r}"
        )
    return ReportIdentity(repopilot_version=repopilot_version, schema_version=schema_version)


def validate_report_consistency(
    cli_version: str,
    default_identity: ReportIdentity,
    strict_identity: ReportIdentity,
) -> None:
    for profile, identity in (("default", default_identity), ("strict", strict_identity)):
        if identity.repopilot_version != cli_version:
            raise ScannerProvenanceError(
                f"{profile} report version {identity.repopilot_version} "
                f"does not match --version {cli_version}"
            )
    if default_identity.schema_version != strict_identity.schema_version:
        raise ScannerProvenanceError(
            "default and strict reports use different schema versions: "
            f"{default_identity.schema_version} vs {strict_identity.schema_version}"
        )


def validate_workspace_version(
    mode: str,
    scanner_version: str,
    workspace_version: str,
    allow_version_mismatch: bool,
) -> bool:
    if scanner_version == workspace_version:
        return False
    if mode == "explicit" and allow_version_mismatch:
        return True
    hint = "\npass --allow-version-mismatch to intentionally test another version" if mode == "explicit" else ""
    raise ScannerProvenanceError(
        f"scanner version {scanner_version} does not match workspace Cargo.toml "
        f"version {workspace_version}{hint}"
    )


def workspace_git_state(repo_root: Path) -> tuple[str | None, bool | None]:
    commit_proc = run(["git", "rev-parse", "HEAD"], cwd=repo_root, check=False)
    commit = commit_proc.stdout.strip() if commit_proc.returncode == 0 else None
    status_proc = run(["git", "status", "--porcelain"], cwd=repo_root, check=False)
    dirty = bool(status_proc.stdout.strip()) if status_proc.returncode == 0 else None
    return commit, dirty


def scan_repo(scanner: ScannerProvenance, path: Path, profile: str) -> dict[str, Any]:
    proc = run(
        [*scanner.command, "scan", str(path), "--format", "json", "--profile", profile],
        check=False,
    )
    if proc.returncode not in (0, 1):
        raise SystemExit(f"scan failed for {path} ({profile}):\n{completed_diagnostic(proc)}")
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"scan produced invalid JSON for {path} ({profile}): {exc}") from exc
