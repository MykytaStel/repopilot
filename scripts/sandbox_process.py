"""Bounded process and Docker execution primitives for the sandbox."""

from __future__ import annotations

import hashlib
import os
import shutil
import signal
import subprocess
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Protocol

from sandbox_contract import ResourcePolicy


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sandbox_env() -> dict[str, str]:
    return {"PATH": os.environ.get("PATH", ""), "LANG": "C.UTF-8", "LC_ALL": "C.UTF-8"}


def _file_sha256(handle: Any) -> str:
    handle.seek(0)
    digest = hashlib.sha256()
    for block in iter(lambda: handle.read(1024 * 1024), b""):
        digest.update(block)
    return digest.hexdigest()


@dataclass(frozen=True)
class CommandResult:
    status: str
    returncode: int | None
    stdout_sha256: str
    stderr_sha256: str
    wall_ms: float
    reason: str | None = None
    payload: bytes | None = None
    stdout_truncated: bool = False
    stderr_truncated: bool = False

    def as_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "status": self.status,
            "returncode": self.returncode,
            "stdout_sha256": self.stdout_sha256,
            "stderr_sha256": self.stderr_sha256,
            "wall_ms": self.wall_ms,
            "resource": {
                "status": "unavailable",
                "reason": "sandbox RSS sampler is not configured",
            },
            "stdout_truncated": self.stdout_truncated,
            "stderr_truncated": self.stderr_truncated,
        }
        if self.reason:
            data["reason"] = self.reason
        return data


@dataclass(frozen=True)
class DockerResult(CommandResult):
    """Result returned by the measured Docker adapter."""


class DockerAdapter(Protocol):
    def run(
        self,
        command: tuple[str, ...],
        cwd: Path,
        image: str,
        timeout_seconds: int,
        policy: ResourcePolicy,
        run_id: str,
    ) -> DockerResult: ...


def _terminate(process: subprocess.Popen[bytes]) -> None:
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=2)
    except (ProcessLookupError, subprocess.TimeoutExpired):
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            pass


def run_command(
    command: tuple[str, ...],
    cwd: Path,
    timeout_seconds: float,
    label: str,
    log_limit_bytes: int = 65536,
    capture_output: bool = False,
) -> CommandResult:
    started = time.perf_counter()
    stdout_file = tempfile.TemporaryFile()
    stderr_file = tempfile.TemporaryFile()
    try:
        process = subprocess.Popen(
            list(command),
            cwd=cwd,
            stdout=stdout_file,
            stderr=stderr_file,
            start_new_session=True,
            env=_sandbox_env(),
        )
    except FileNotFoundError:
        stdout_file.close()
        stderr_file.close()
        return CommandResult(
            "unavailable",
            None,
            sha256_bytes(b""),
            sha256_bytes(b""),
            0.0,
            "program is unavailable",
        )
    try:
        process.wait(timeout=timeout_seconds)
    except subprocess.TimeoutExpired:
        _terminate(process)
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            pass
        status, returncode, reason = (
            "timeout",
            None,
            f"{label} exceeded {timeout_seconds:g}s timeout",
        )
    else:
        status = "passed" if process.returncode == 0 else "failed"
        returncode, reason = process.returncode, None
    wall_ms = round((time.perf_counter() - started) * 1000, 3)
    stdout_sha = _file_sha256(stdout_file)
    stderr_sha = _file_sha256(stderr_file)
    stdout_file.seek(0, 2)
    stdout_truncated = stdout_file.tell() > log_limit_bytes
    stderr_file.seek(0, 2)
    stderr_truncated = stderr_file.tell() > log_limit_bytes
    stdout_file.seek(0)
    stderr_file.seek(0)
    payload = stdout_file.read(log_limit_bytes) if capture_output else None
    stderr_file.read(log_limit_bytes)
    stdout_file.close()
    stderr_file.close()
    return CommandResult(
        status,
        returncode,
        stdout_sha,
        stderr_sha,
        wall_ms,
        reason,
        payload,
        stdout_truncated,
        stderr_truncated,
    )


class SubprocessDockerAdapter:
    """Run a command in a pinned image with an explicit measured policy."""

    def __init__(self, docker_binary: str = "docker") -> None:
        self.docker_binary = docker_binary

    def run(
        self,
        command: tuple[str, ...],
        cwd: Path,
        image: str,
        timeout_seconds: int,
        policy: ResourcePolicy,
        run_id: str,
    ) -> DockerResult:
        if shutil.which(self.docker_binary) is None:
            return DockerResult(
                "unavailable",
                None,
                sha256_bytes(b""),
                sha256_bytes(b""),
                0.0,
                "docker CLI is unavailable",
            )
        probe = run_command(
            (self.docker_binary, "info", "--format", "{{.ServerVersion}}"),
            cwd,
            5,
            "docker daemon probe",
            4096,
        )
        if probe.status != "passed":
            return DockerResult(
                "unavailable",
                probe.returncode,
                probe.stdout_sha256,
                probe.stderr_sha256,
                probe.wall_ms,
                "docker daemon is unavailable",
            )
        image_probe = run_command(
            (
                self.docker_binary,
                "image",
                "inspect",
                image,
                "--format",
                "{{.Id}}",
            ),
            cwd,
            5,
            "docker image probe",
            4096,
        )
        if image_probe.status != "passed":
            return DockerResult(
                "unavailable",
                image_probe.returncode,
                image_probe.stdout_sha256,
                image_probe.stderr_sha256,
                image_probe.wall_ms,
                "pinned Docker image is unavailable locally",
            )
        name = f"repopilot-sandbox-{run_id}"
        docker_command = self.build_command(
            command, cwd, image, policy, name, self.docker_binary
        )
        result = run_command(
            tuple(docker_command),
            cwd,
            timeout_seconds,
            "docker command",
            policy.log_limit_bytes,
        )
        if result.status == "timeout":
            self._remove_container(name)
        return DockerResult(**result.__dict__)

    @staticmethod
    def build_command(
        command: tuple[str, ...],
        cwd: Path,
        image: str,
        policy: ResourcePolicy,
        name: str,
        docker_binary: str = "docker",
    ) -> list[str]:
        return [
            docker_binary,
            "run",
            "--rm",
            "--pull=never",
            "--name",
            name,
            "--network",
            policy.network,
            "--cpus",
            str(policy.cpus),
            "--memory",
            f"{policy.memory_mb}m",
            "--pids-limit",
            "512",
            "--cap-drop",
            "ALL",
            "--security-opt",
            "no-new-privileges",
            "--mount",
            f"type=bind,src={cwd},dst=/workspace",
            "--workdir",
            "/workspace",
            image,
            *command,
        ]

    def _remove_container(self, name: str) -> None:
        try:
            subprocess.run(
                [self.docker_binary, "rm", "-f", name],
                capture_output=True,
                check=False,
                timeout=5,
            )
        except (OSError, subprocess.TimeoutExpired):
            pass
