"""TOML parser for the local real-project sandbox manifest."""

from __future__ import annotations

import hashlib
import tomllib
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

from sandbox_contract import (
    CASE_ID,
    IMAGE,
    SHA1,
    PROTOCOL,
    SCHEMA_VERSION,
    ALLOWED_PROGRAMS,
    ResourcePolicy,
    SandboxCase,
    SandboxManifest,
    SandboxManifestError,
    SandboxProject,
)


def _string(raw: dict[str, Any], field: str, context: str) -> str:
    value = raw.get(field)
    if not isinstance(value, str) or not value.strip():
        raise SandboxManifestError(f"{context}: {field} must be a non-empty string")
    return value.strip()


def _positive_int(raw: dict[str, Any], field: str, context: str) -> int:
    value = raw.get(field)
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise SandboxManifestError(f"{context}: {field} must be a positive integer")
    return value


def _command(
    raw: object, field: str, context: str, required: bool = True
) -> tuple[str, ...]:
    if raw is None and not required:
        return ()
    if (
        not isinstance(raw, list)
        or not raw
        or not all(isinstance(item, str) and item for item in raw)
    ):
        raise SandboxManifestError(
            f"{context}: {field} must be a non-empty string array"
        )
    command = tuple(raw)
    program = Path(command[0]).name
    if program not in ALLOWED_PROGRAMS:
        raise SandboxManifestError(f"{context}: {field} program is not allowlisted")
    if any(
        any(token in item for token in (";", "&&", "||", "`", "$(", "\n", "\r"))
        for item in command
    ):
        raise SandboxManifestError(f"{context}: {field} contains shell metacharacters")
    return command


def _load_document(path: Path) -> tuple[dict[str, Any], str]:
    try:
        raw_bytes = path.read_bytes()
        document = tomllib.loads(raw_bytes.decode("utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        raise SandboxManifestError(
            f"cannot read sandbox manifest {path}: {error}"
        ) from error
    if not isinstance(document, dict):
        raise SandboxManifestError("sandbox manifest must be a TOML table")
    return document, hashlib.sha256(raw_bytes).hexdigest()


def load_manifest(path: Path) -> SandboxManifest:
    document, manifest_hash = _load_document(path)
    allowed_top = {
        "schema_version",
        "corpus",
        "protocol",
        "resource_policy",
        "project",
        "case",
    }
    unknown = set(document) - allowed_top
    if unknown:
        raise SandboxManifestError(
            f"unknown top-level fields: {', '.join(sorted(unknown))}"
        )
    if document.get("schema_version") != SCHEMA_VERSION:
        raise SandboxManifestError(f"schema_version must be {SCHEMA_VERSION}")
    protocol = document.get("protocol")
    if protocol != PROTOCOL:
        raise SandboxManifestError(f"protocol must be {PROTOCOL}")
    corpus = _string(document, "corpus", "manifest")
    policy = _parse_policy(document.get("resource_policy"))
    projects = _parse_projects(document.get("project"))
    project_ids = {project.project_id for project in projects}
    cases = _parse_cases(document.get("case"), project_ids)
    return SandboxManifest(
        SCHEMA_VERSION,
        corpus,
        protocol,
        policy,
        tuple(projects),
        tuple(cases),
        path,
        manifest_hash,
    )


def _parse_policy(raw: object) -> ResourcePolicy:
    if not isinstance(raw, dict):
        raise SandboxManifestError("resource_policy must be a table")
    allowed = {
        "schema_version",
        "policy_id",
        "max_concurrent",
        "cpus",
        "memory_mb",
        "measured_timeout_seconds",
        "prepare_timeout_seconds",
        "network",
        "log_limit_bytes",
    }
    unknown = set(raw) - allowed
    if unknown:
        raise SandboxManifestError(
            f"resource_policy has unknown fields: {', '.join(sorted(unknown))}"
        )
    cpus = raw.get("cpus")
    if (
        not isinstance(cpus, (int, float))
        or isinstance(cpus, bool)
        or cpus <= 0
        or cpus > 2
    ):
        raise SandboxManifestError("resource_policy: cpus must be in (0, 2]")
    network = raw.get("network")
    if network != "none":
        raise SandboxManifestError(
            "resource_policy: network must be 'none' for measured runs"
        )
    if raw.get("schema_version") != 1:
        raise SandboxManifestError("resource_policy: schema_version must be 1")
    max_concurrent = _positive_int(raw, "max_concurrent", "resource_policy")
    memory_mb = _positive_int(raw, "memory_mb", "resource_policy")
    measured_timeout = _positive_int(raw, "measured_timeout_seconds", "resource_policy")
    prepare_timeout = _positive_int(raw, "prepare_timeout_seconds", "resource_policy")
    log_limit = _positive_int(raw, "log_limit_bytes", "resource_policy")
    if max_concurrent != 1:
        raise SandboxManifestError("resource_policy: max_concurrent must be 1")
    if memory_mb > 4096:
        raise SandboxManifestError("resource_policy: memory_mb must be at most 4096")
    if measured_timeout > 300:
        raise SandboxManifestError(
            "resource_policy: measured_timeout_seconds must be at most 300"
        )
    if prepare_timeout > 1200:
        raise SandboxManifestError(
            "resource_policy: prepare_timeout_seconds must be at most 1200"
        )
    return ResourcePolicy(
        1,
        _string(raw, "policy_id", "resource_policy"),
        max_concurrent,
        float(cpus),
        memory_mb,
        measured_timeout,
        prepare_timeout,
        "none",
        log_limit,
    )


def _parse_projects(raw: object) -> list[SandboxProject]:
    if not isinstance(raw, list) or not raw:
        raise SandboxManifestError("manifest must contain projects")
    projects: list[SandboxProject] = []
    ids: set[str] = set()
    for index, item in enumerate(raw, start=1):
        if not isinstance(item, dict):
            raise SandboxManifestError(f"project {index}: entry must be a table")
        allowed = {"id", "name", "url", "sha", "language", "framework"}
        unknown = set(item) - allowed
        if unknown:
            raise SandboxManifestError(
                f"project {index}: unknown fields: {', '.join(sorted(unknown))}"
            )
        project_id = _string(item, "id", f"project {index}")
        sha = _string(item, "sha", f"project {project_id}")
        url = _string(item, "url", f"project {project_id}")
        if project_id in ids:
            raise SandboxManifestError(f"duplicate project id {project_id}")
        if not SHA1.fullmatch(sha):
            raise SandboxManifestError(
                f"project {project_id}: sha must be a full lowercase SHA-1"
            )
        parsed_url = urlparse(url)
        if (
            parsed_url.scheme != "https"
            or not parsed_url.netloc
            or parsed_url.query
            or parsed_url.fragment
            or any(char in url for char in ("\n", "\r", '"'))
        ):
            raise SandboxManifestError(
                f"project {project_id}: url must be an https URL"
            )
        ids.add(project_id)
        projects.append(
            SandboxProject(
                project_id,
                _string(item, "name", f"project {project_id}"),
                url,
                sha,
                _string(item, "language", f"project {project_id}"),
                str(item.get("framework") or ""),
            )
        )
    return projects


def _parse_cases(raw: object, project_ids: set[str]) -> list[SandboxCase]:
    if not isinstance(raw, list) or not raw:
        raise SandboxManifestError("manifest must contain cases")
    cases: list[SandboxCase] = []
    ids: set[str] = set()
    for index, item in enumerate(raw, start=1):
        if not isinstance(item, dict):
            raise SandboxManifestError(f"case {index}: entry must be a table")
        allowed = {
            "id",
            "project",
            "image",
            "setup",
            "oracle",
            "expected_oracle",
            "patch",
            "mutation_kind",
            "split",
        }
        unknown = set(item) - allowed
        if unknown:
            raise SandboxManifestError(
                f"case {index}: unknown fields: {', '.join(sorted(unknown))}"
            )
        case_id = _string(item, "id", f"case {index}")
        project_id = _string(item, "project", f"case {case_id}")
        image = _string(item, "image", f"case {case_id}")
        expected = _string(item, "expected_oracle", f"case {case_id}")
        if not CASE_ID.fullmatch(case_id) or case_id in ids:
            raise SandboxManifestError(f"case {case_id}: invalid or duplicate id")
        if project_id not in project_ids:
            raise SandboxManifestError(f"case {case_id}: unknown project {project_id}")
        if not IMAGE.fullmatch(image):
            raise SandboxManifestError(
                f"case {case_id}: image must include a sha256 digest"
            )
        if expected not in {"passed", "failed", "unavailable"}:
            raise SandboxManifestError(
                f"case {case_id}: expected_oracle is unsupported"
            )
        patch = item.get("patch")
        if patch is not None and (not isinstance(patch, str) or not patch.strip()):
            raise SandboxManifestError(
                f"case {case_id}: patch must be a non-empty path"
            )
        mutation_kind = item.get("mutation_kind", "control")
        split = item.get("split", "pilot")
        if mutation_kind not in {"control", "violation", "negative-control"}:
            raise SandboxManifestError(f"case {case_id}: mutation_kind is unsupported")
        if split not in {"pilot", "tuning", "evaluation"}:
            raise SandboxManifestError(f"case {case_id}: split is unsupported")
        if mutation_kind != "control" and patch is None:
            raise SandboxManifestError(
                f"case {case_id}: mutation cases require a patch"
            )
        ids.add(case_id)
        cases.append(
            SandboxCase(
                case_id,
                project_id,
                image,
                _command(item.get("setup"), "setup", f"case {case_id}", False),
                _command(item.get("oracle"), "oracle", f"case {case_id}"),
                expected,
                patch.strip() if isinstance(patch, str) else None,
                mutation_kind,
                split,
            )
        )
    return cases
