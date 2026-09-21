"""Portable best-effort peak RSS sampling for differential commands."""

from __future__ import annotations

import re
import subprocess
import sys
import tempfile
import time
from functools import lru_cache
from pathlib import Path
from typing import Any


RESOURCE_SOURCE = "posix-time-v1"
_LINUX_RSS = re.compile(r"Maximum resident set size \(kbytes\):\s*(?P<rss>\d+)")
_DARWIN_RSS = re.compile(r"^\s*(?P<rss>\d+)\s+maximum resident set size\s*$", re.MULTILINE)


def parse_peak_rss_kb(output: str, platform: str) -> int | None:
    """Parse `/usr/bin/time` peak RSS output into KiB."""

    pattern = _DARWIN_RSS if platform == "darwin" else _LINUX_RSS if platform == "linux" else None
    if pattern is None:
        return None
    match = pattern.search(output)
    if match is None:
        return None
    value = int(match.group("rss"))
    return (value + 1023) // 1024 if platform == "darwin" else value


def resource_command(
    command: tuple[str, ...], output_path: Path, platform: str | None = None
) -> list[str] | None:
    """Return a `/usr/bin/time` command, or None when the platform cannot sample RSS."""

    system = platform or sys.platform
    if system not in {"darwin", "linux"} or not Path("/usr/bin/time").is_file():
        return None
    flags = ["-l"] if system == "darwin" else ["-v"]
    return ["/usr/bin/time", *flags, "-o", str(output_path), *command]


def resource_sample_from_file(path: Path, platform: str | None = None) -> dict[str, Any]:
    """Read a sampler output file and return an explicit measured/unavailable record."""

    try:
        output = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return {
            "status": "unavailable",
            "reason": "portable peak RSS sampler output is unavailable",
            "source": RESOURCE_SOURCE,
        }
    sample = parse_peak_rss_kb(output, platform or sys.platform)
    if sample is None or sample <= 0:
        return {
            "status": "unavailable",
            "reason": "portable peak RSS sampler emitted no positive sample",
            "source": RESOURCE_SOURCE,
        }
    return {"status": "available", "peak_rss_kb": sample, "source": RESOURCE_SOURCE}


def unavailable_resource(reason: str) -> dict[str, Any]:
    return {"status": "unavailable", "reason": reason, "source": "unavailable"}


@lru_cache(maxsize=1)
def _resource_sampler_supported() -> bool:
    """Probe the optional sampler without allowing it to mask child status."""

    probe_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(prefix="repopilot-rss-probe-", suffix=".txt", delete=False) as handle:
            probe_path = Path(handle.name)
        wrapped = resource_command((sys.executable, "-c", "pass"), probe_path)
        if wrapped is None:
            return False
        process = subprocess.run(wrapped, capture_output=True, check=False, timeout=5)
        return process.returncode == 0 and resource_sample_from_file(probe_path)["status"] == "available"
    except (OSError, subprocess.TimeoutExpired):
        return False
    finally:
        if probe_path is not None:
            try:
                probe_path.unlink()
            except OSError:
                pass


def execute_timed_command(
    command: tuple[str, ...], cwd: Path, timeout_seconds: float
) -> dict[str, Any]:
    """Run a command and return process, timing, and resource observations."""

    sampler_path: Path | None = None
    wrapped_command: list[str] | None = None
    if _resource_sampler_supported():
        with tempfile.NamedTemporaryFile(prefix="repopilot-rss-", suffix=".txt", delete=False) as handle:
            sampler_path = Path(handle.name)
        wrapped_command = resource_command(command, sampler_path)
    started = time.perf_counter()
    try:
        process = subprocess.run(
            wrapped_command or list(command),
            cwd=cwd,
            capture_output=True,
            check=False,
            timeout=timeout_seconds,
        )
    except FileNotFoundError:
        status, returncode, stdout, stderr = "unavailable", None, b"", b""
    except subprocess.TimeoutExpired as error:
        status, returncode = "timeout", None
        stdout = error.stdout or b""
        stderr = error.stderr or b""
    else:
        status = "passed" if process.returncode == 0 else "failed"
        returncode, stdout, stderr = process.returncode, process.stdout, process.stderr
    finally:
        wall_ms = round((time.perf_counter() - started) * 1000, 3)
    if status == "timeout":
        resource = unavailable_resource("command timed out before peak RSS sampling completed")
    elif sampler_path is None:
        resource = unavailable_resource("portable peak RSS sampler is unavailable on this platform")
    else:
        resource = resource_sample_from_file(sampler_path)
    if sampler_path is not None:
        try:
            sampler_path.unlink()
        except OSError:
            pass
    return {
        "status": status,
        "returncode": returncode,
        "stdout": stdout,
        "stderr": stderr,
        "wall_ms": wall_ms,
        "resource": resource,
    }
