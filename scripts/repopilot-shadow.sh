#!/usr/bin/env bash
set -euo pipefail

exec python3 - "$@" <<'PY'
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "repopilot-shadow-1"
MAX_LOG_BYTES = 65_536
PROFILES = {"default", "strict"}
PRIORITIES = {"p0", "p1", "p2", "p3"}
SENSITIVE_ASSIGNMENT = re.compile(
    r"(?i)(\b(?:secret|token|password|authorization|api[_-]?key|private[_-]?key)\b\s*[:=]\s*)([^\s,;]+)"
)
BEARER = re.compile(r"(?i)(\bBearer\s+)([A-Za-z0-9._~+/=-]+)")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Record a non-blocking RepoPilot policy observation.")
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--repo", type=Path, default=Path("."))
    parser.add_argument("--binary", default="repopilot")
    parser.add_argument("--profile", default="strict")
    parser.add_argument("--fail-on-priority", default="p1")
    parser.add_argument("--timeout-seconds", type=int, default=300)
    parser.add_argument("--config", type=Path)
    return parser.parse_args()


def sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def redact(text: str) -> str:
    text = SENSITIVE_ASSIGNMENT.sub(r"\1[REDACTED_SECRET]", text)
    return BEARER.sub(r"\1[REDACTED_SECRET]", text)


def write_log(path: Path, content: bytes) -> None:
    text = redact(content.decode("utf-8", errors="replace"))
    if len(text.encode("utf-8")) > MAX_LOG_BYTES:
        encoded = text.encode("utf-8")[:MAX_LOG_BYTES]
        text = encoded.decode("utf-8", errors="ignore") + "\n[output truncated]\n"
    path.write_text(text, encoding="utf-8")


def write_json(path: Path, data: dict[str, Any]) -> None:
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", dir=path.parent, prefix=f".{path.name}.", delete=False
    ) as handle:
        json.dump(data, handle, indent=2, sort_keys=True)
        handle.write("\n")
        temporary = Path(handle.name)
    os.replace(temporary, path)


def resolve_binary(value: str) -> str | None:
    if "/" in value:
        candidate = Path(value)
        if not candidate.is_absolute():
            candidate = Path.cwd() / candidate
        candidate = candidate.resolve()
        return str(candidate) if candidate.is_file() and os.access(candidate, os.X_OK) else None
    return shutil.which(value)


def revision(repo: Path) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo), "rev-parse", "HEAD"],
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.TimeoutExpired):
        return "unavailable"
    value = result.stdout.strip()
    return value if result.returncode == 0 and value else "unavailable"


def analyzer_version(binary: str) -> str:
    try:
        result = subprocess.run(
            [binary, "--version"],
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired):
        return "unavailable"
    output = (result.stdout or result.stderr).strip().splitlines()
    return redact(output[0]) if output else "unavailable"


def run_analyzer(command: list[str], repo: Path, timeout_seconds: int) -> tuple[int | None, bytes, bytes, str | None]:
    try:
        process = subprocess.Popen(
            command,
            cwd=repo,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=(os.name == "posix"),
        )
    except OSError as error:
        return None, b"", str(error).encode("utf-8"), f"analyzer could not start: {error}"

    try:
        stdout, stderr = process.communicate(timeout=timeout_seconds)
    except subprocess.TimeoutExpired:
        if os.name == "posix":
            os.killpg(process.pid, signal.SIGKILL)
        else:
            process.kill()
        stdout, stderr = process.communicate()
        return None, stdout, stderr, f"analyzer timed out after {timeout_seconds} seconds"
    return process.returncode, stdout, stderr, None


def valid_report(report: Path) -> tuple[bool, str | None]:
    if not report.is_file():
        return False, "analyzer did not produce report.json"
    try:
        value = json.loads(report.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        return False, f"report JSON is invalid: {error}"
    if not isinstance(value, dict):
        return False, "report JSON must be an object"
    return True, None


def main() -> int:
    args = parse_args()
    output_dir = args.output_dir.expanduser()
    try:
        if output_dir.exists() and not output_dir.is_dir():
            raise OSError(f"output path is not a directory: {output_dir}")
        output_dir.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        print(f"repopilot-shadow: cannot prepare output directory: {error}", file=sys.stderr)
        return 2

    report = output_dir / "report.json"
    metadata_path = output_dir / "shadow.json"
    stdout_path = output_dir / "stdout.log"
    stderr_path = output_dir / "stderr.log"
    binary = resolve_binary(args.binary)
    config_hash = sha256_file(args.config) if args.config else None
    metadata: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "status": "unavailable",
        "policy": {"profile": args.profile, "fail_on_priority": args.fail_on_priority},
        "repository_revision": revision(args.repo),
        "analyzer_version": analyzer_version(binary) if binary else "unavailable",
        "config_sha256": config_hash,
        "exit_code": None,
        "report_sha256": None,
        "report_path": "report.json",
        "reason": "shadow runner did not start",
    }

    if args.profile not in PROFILES or args.fail_on_priority not in PRIORITIES or args.timeout_seconds <= 0:
        metadata["status"] = "invalid"
        metadata["reason"] = "invalid shadow policy or timeout"
        write_log(stdout_path, b"")
        write_log(stderr_path, b"")
        write_json(metadata_path, metadata)
        return 0

    if args.config and config_hash is None:
        metadata["status"] = "unavailable"
        metadata["reason"] = "configured file does not exist"
        write_log(stdout_path, b"")
        write_log(stderr_path, b"")
        write_json(metadata_path, metadata)
        return 0

    if binary is None:
        metadata["reason"] = "analyzer executable is unavailable"
        write_log(stdout_path, b"")
        write_log(stderr_path, b"")
        write_json(metadata_path, metadata)
        return 0

    command = [
        binary,
        "scan",
        str(args.repo),
        "--format",
        "json",
        "--output",
        str(report),
        "--profile",
        args.profile,
        "--fail-on-priority",
        args.fail_on_priority,
        "--quiet",
        "--no-progress",
    ]
    if args.config:
        command.extend(["--config", str(args.config)])
    exit_code, stdout, stderr, run_reason = run_analyzer(command, args.repo, args.timeout_seconds)
    write_log(stdout_path, stdout)
    write_log(stderr_path, stderr)
    metadata["exit_code"] = exit_code
    metadata["report_sha256"] = sha256_file(report)

    if run_reason:
        metadata["reason"] = run_reason
    else:
        report_is_valid, reason = valid_report(report)
        if not report_is_valid:
            metadata["status"] = "invalid"
            metadata["reason"] = reason
        elif exit_code == 0:
            metadata["status"] = "passed"
            metadata["reason"] = "candidate policy passed"
        else:
            metadata["status"] = "failed"
            metadata["reason"] = f"candidate policy exited with status {exit_code}"

    write_json(metadata_path, metadata)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
PY
