"""Bounded peak RSS sampling for sandbox commands."""

from __future__ import annotations

import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from differential_resource import (
    resource_command,
    resource_sample_from_file,
    unavailable_resource,
)


@dataclass
class ResourceProbe:
    command: tuple[str, ...]
    output_path: Path | None
    unavailable_reason: str | None = None

    def sample(self, status: str, reason: str | None = None) -> dict[str, Any]:
        if status == "timeout":
            return unavailable_resource(
                "command timed out before peak RSS sampling completed"
            )
        if reason is not None:
            return unavailable_resource(reason)
        if self.output_path is None:
            return unavailable_resource(
                reason
                or self.unavailable_reason
                or "sandbox RSS sampler is unavailable"
            )
        return resource_sample_from_file(self.output_path)

    def cleanup(self) -> None:
        if self.output_path is not None:
            try:
                self.output_path.unlink(missing_ok=True)
            except OSError:
                pass


def prepare_resource_probe(command: tuple[str, ...]) -> ResourceProbe:
    """Wrap a command when the host exposes the portable RSS sampler."""

    output_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            prefix="repopilot-sandbox-rss-", suffix=".txt", delete=False
        ) as handle:
            output_path = Path(handle.name)
        sampled = resource_command(command, output_path)
        if sampled is None:
            output_path.unlink(missing_ok=True)
            return ResourceProbe(
                command,
                None,
                "sandbox RSS sampler is unavailable on this platform",
            )
        return ResourceProbe(tuple(sampled), output_path)
    except OSError:
        if output_path is not None:
            output_path.unlink(missing_ok=True)
        return ResourceProbe(
            command,
            None,
            "sandbox RSS sampler could not be initialized",
        )
