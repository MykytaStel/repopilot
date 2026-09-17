"""Manifest and result contracts for the local real-project sandbox."""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path


SCHEMA_VERSION = 1
PROTOCOL = "repopilot-validation-sandbox-v1"
SHA256 = re.compile(r"^[0-9a-f]{64}$")
SHA1 = re.compile(r"^[0-9a-f]{40}$")
CASE_ID = re.compile(r"^[a-z0-9][a-z0-9._-]+$")
IMAGE = re.compile(r"^[^@\s]+@sha256:[0-9a-f]{64}$")
ALLOWED_PROGRAMS = {
    "cargo",
    "dotnet",
    "go",
    "gradle",
    "make",
    "mvn",
    "node",
    "npm",
    "pytest",
    "python",
    "python3",
}
ALLOWED_PHASES = {
    "prepare",
    "baseline",
    "mutate",
    "analyze",
    "oracle",
    "revert",
    "collect",
}
ALLOWED_STATUSES = {"passed", "failed", "unavailable", "timeout", "skipped", "dry-run"}


class SandboxManifestError(ValueError):
    """Raised when a sandbox manifest or result violates the protocol."""


@dataclass(frozen=True)
class ResourcePolicy:
    schema_version: int
    policy_id: str
    max_concurrent: int
    cpus: float
    memory_mb: int
    measured_timeout_seconds: int
    prepare_timeout_seconds: int
    network: str
    log_limit_bytes: int


@dataclass(frozen=True)
class SandboxProject:
    project_id: str
    name: str
    url: str
    sha: str
    language: str
    framework: str


@dataclass(frozen=True)
class SandboxCase:
    case_id: str
    project_id: str
    image: str
    setup: tuple[str, ...]
    oracle: tuple[str, ...]
    expected_oracle: str
    patch: str | None
    mutation_kind: str = "control"
    split: str = "pilot"
    analysis_mode: str = "default"


@dataclass(frozen=True)
class SandboxManifest:
    schema_version: int
    corpus: str
    protocol: str
    policy: ResourcePolicy
    projects: tuple[SandboxProject, ...]
    cases: tuple[SandboxCase, ...]
    path: Path
    sha256: str

    def project(self, project_id: str) -> SandboxProject:
        for project in self.projects:
            if project.project_id == project_id:
                return project
        raise SandboxManifestError(f"unknown project {project_id!r}")

    def case(self, case_id: str) -> SandboxCase:
        for case in self.cases:
            if case.case_id == case_id:
                return case
        raise SandboxManifestError(f"unknown case {case_id!r}")


def load_manifest(path: Path) -> SandboxManifest:
    from sandbox_manifest import load_manifest as _load_manifest

    return _load_manifest(path)


def validate_artifact(path: Path, manifest_path: Path) -> dict[str, object]:
    from sandbox_artifact import validate_artifact as _validate_artifact

    return _validate_artifact(path, manifest_path)


def validate_pilot_summary(path: Path, manifest_path: Path) -> dict[str, object]:
    from sandbox_artifact import validate_pilot_summary as _validate_pilot_summary

    return _validate_pilot_summary(path, manifest_path)


def validate_mutation_summary(path: Path, manifest_path: Path) -> dict[str, object]:
    from sandbox_artifact import validate_mutation_summary as _validate_mutation_summary

    return _validate_mutation_summary(path, manifest_path)
