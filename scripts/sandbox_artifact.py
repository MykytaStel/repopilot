"""Validation for canonical sandbox result artifacts."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from sandbox_contract import (
    ALLOWED_PHASES,
    ALLOWED_STATUSES,
    PROTOCOL,
    SCHEMA_VERSION,
    SHA1,
    SHA256,
    SandboxManifestError,
)
from sandbox_manifest import load_manifest


def validate_artifact(path: Path, manifest_path: Path) -> dict[str, Any]:
    manifest = load_manifest(manifest_path)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise SandboxManifestError(
            f"cannot read sandbox artifact {path}: {error}"
        ) from error
    if not isinstance(data, dict):
        raise SandboxManifestError("sandbox artifact must be a JSON object")
    if data.get("schema_version") != SCHEMA_VERSION or data.get("protocol") != PROTOCOL:
        raise SandboxManifestError("sandbox artifact schema/protocol mismatch")
    if data.get("manifest_sha256") != manifest.sha256:
        raise SandboxManifestError(
            "sandbox artifact manifest_sha256 does not match manifest"
        )
    case = manifest.case(str(data.get("case_id")))
    if data.get("project_id") != case.project_id:
        raise SandboxManifestError("sandbox artifact project does not match case")
    if data.get("status") not in ALLOWED_STATUSES:
        raise SandboxManifestError("sandbox artifact has unsupported status")
    phases = data.get("phases")
    if not isinstance(phases, list) or any(
        not isinstance(phase, dict)
        or phase.get("name") not in ALLOWED_PHASES
        or phase.get("status") not in ALLOWED_STATUSES
        for phase in phases
    ):
        raise SandboxManifestError("sandbox artifact phases are invalid")
    cleanup = data.get("cleanup")
    if not isinstance(cleanup, dict) or cleanup.get("status") not in {
        "complete",
        "partial",
        "failed",
    }:
        raise SandboxManifestError("sandbox artifact requires a cleanup receipt")
    inputs = data.get("inputs")
    if not isinstance(inputs, dict):
        raise SandboxManifestError("sandbox artifact requires input provenance")
    provenance = data.get("provenance")
    if provenance is not None:
        if not isinstance(provenance, dict):
            raise SandboxManifestError("sandbox artifact provenance is invalid")
        analysis_mode = provenance.get("analysis_mode", "default")
        if analysis_mode not in {"default", "changed"}:
            raise SandboxManifestError("sandbox artifact analysis_mode is unsupported")
        if analysis_mode != case.analysis_mode:
            raise SandboxManifestError(
                "sandbox artifact analysis_mode does not match manifest"
            )
        profile = provenance.get("profile", "default")
        if profile not in {"default", "strict"}:
            raise SandboxManifestError("sandbox artifact profile is unsupported")
        if profile != case.profile:
            raise SandboxManifestError("sandbox artifact profile does not match manifest")
    project = manifest.project(case.project_id)
    if inputs.get("project_sha") != project.sha or inputs.get("image") != case.image:
        raise SandboxManifestError(
            "sandbox artifact project SHA or image does not match manifest"
        )
    source_sha = inputs.get("source_sha")
    if source_sha is not None and (
        not isinstance(source_sha, str) or not SHA1.fullmatch(source_sha)
    ):
        raise SandboxManifestError("sandbox artifact source_sha is invalid")
    patch_sha = inputs.get("patch_sha256")
    if patch_sha is not None and (
        not isinstance(patch_sha, str) or not SHA256.fullmatch(patch_sha)
    ):
        raise SandboxManifestError("sandbox artifact patch_sha256 is invalid")
    return {
        "status": "valid",
        "case_id": case.case_id,
        "project_id": case.project_id,
        "phases": len(phases),
    }


def validate_pilot_summary(path: Path, manifest_path: Path) -> dict[str, Any]:
    manifest = load_manifest(manifest_path)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise SandboxManifestError(
            f"cannot read pilot summary {path}: {error}"
        ) from error
    if not isinstance(data, dict):
        raise SandboxManifestError("pilot summary must be a JSON object")
    if (
        data.get("schema_version") != 1
        or data.get("kind") != "pilot-summary"
        or data.get("protocol") != PROTOCOL
        or data.get("manifest_sha256") != manifest.sha256
    ):
        raise SandboxManifestError(
            "pilot summary schema, protocol or manifest mismatch"
        )
    repeats = data.get("repeats")
    if (
        not isinstance(repeats, int)
        or isinstance(repeats, bool)
        or not 1 <= repeats <= 3
    ):
        raise SandboxManifestError("pilot summary repeats must be between 1 and 3")
    if data.get("status") not in {"passed", "drift", "unavailable", "failed"}:
        raise SandboxManifestError("pilot summary has unsupported status")
    cases = data.get("cases")
    if not isinstance(cases, list) or len(cases) != len(manifest.cases):
        raise SandboxManifestError("pilot summary cases do not match manifest")
    expected_ids = {case.case_id for case in manifest.cases}
    seen: set[str] = set()
    for item in cases:
        if not isinstance(item, dict):
            raise SandboxManifestError("pilot summary case is invalid")
        case_id = item.get("case_id")
        if (
            not isinstance(case_id, str)
            or case_id not in expected_ids
            or case_id in seen
        ):
            raise SandboxManifestError("pilot summary case id is invalid or duplicated")
        seen.add(case_id)
        if item.get("status") not in {"passed", "drift", "unavailable"}:
            raise SandboxManifestError("pilot summary case has unsupported status")
        if item.get("analysis_mode", "default") != manifest.case(case_id).analysis_mode:
            raise SandboxManifestError(
                "pilot summary analysis_mode does not match manifest"
            )
        if item.get("profile", "default") != manifest.case(case_id).profile:
            raise SandboxManifestError("pilot summary profile does not match manifest")
        runs = item.get("runs")
        if not isinstance(runs, list) or len(runs) != repeats:
            raise SandboxManifestError("pilot summary run count does not match repeats")
        for run in runs:
            if not isinstance(run, dict) or not isinstance(run.get("artifact"), str):
                raise SandboxManifestError("pilot summary run artifact is invalid")
            artifact = (path.parent / run["artifact"]).resolve()
            try:
                artifact.relative_to(path.parent.resolve())
            except ValueError as error:
                raise SandboxManifestError(
                    "pilot artifact path escapes summary directory"
                ) from error
            validated = validate_artifact(artifact, manifest_path)
            if validated["case_id"] != case_id:
                raise SandboxManifestError(
                    "pilot artifact case does not match summary case"
                )
        comparison = item.get("comparison")
        if not isinstance(comparison, dict) or comparison.get("status") not in {
            "stable",
            "drift",
            "unavailable",
        }:
            raise SandboxManifestError("pilot summary comparison is invalid")
    if seen != expected_ids:
        raise SandboxManifestError("pilot summary is missing manifest cases")
    return {"status": "valid", "cases": len(cases), "repeats": repeats}


def validate_mutation_summary(path: Path, manifest_path: Path) -> dict[str, Any]:
    manifest = load_manifest(manifest_path)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise SandboxManifestError(
            f"cannot read mutation summary {path}: {error}"
        ) from error
    if (
        not isinstance(data, dict)
        or data.get("schema_version") != 1
        or data.get("kind") != "mutation-summary"
        or data.get("protocol") != PROTOCOL
        or data.get("manifest_sha256") != manifest.sha256
    ):
        raise SandboxManifestError(
            "mutation summary schema, protocol or manifest mismatch"
        )
    if data.get("status") not in {"passed", "failed", "unavailable"}:
        raise SandboxManifestError("mutation summary has unsupported status")
    expected_cases = {
        case.case_id: case
        for case in manifest.cases
        if case.mutation_kind in {"violation", "negative-control"}
    }
    cases = data.get("cases")
    if (
        not expected_cases
        or not isinstance(cases, list)
        or len(cases) != len(expected_cases)
    ):
        raise SandboxManifestError("mutation summary cases do not match manifest")
    seen: set[str] = set()
    for item in cases:
        if not isinstance(item, dict):
            raise SandboxManifestError("mutation summary case is invalid")
        case_id = item.get("case_id")
        if (
            not isinstance(case_id, str)
            or case_id not in expected_cases
            or case_id in seen
        ):
            raise SandboxManifestError(
                "mutation summary case id is invalid or duplicated"
            )
        seen.add(case_id)
        expected = expected_cases[case_id]
        if (
            item.get("mutation_kind") != expected.mutation_kind
            or item.get("split") != expected.split
            or item.get("expected_oracle") != expected.expected_oracle
            or item.get("analysis_mode", "default") != expected.analysis_mode
            or item.get("profile", "default") != expected.profile
        ):
            raise SandboxManifestError(
                "mutation summary case metadata does not match manifest"
            )
        expected_rule_ids = item.get("expected_rule_ids")
        if expected.expected_rule_ids:
            if expected_rule_ids != list(expected.expected_rule_ids):
                raise SandboxManifestError(
                    "mutation summary expected rule IDs do not match manifest"
                )
        elif expected_rule_ids is not None and expected_rule_ids != []:
            raise SandboxManifestError(
                "mutation summary expected rule IDs do not match manifest"
            )
        observed_rule_ids = item.get("observed_rule_ids")
        if observed_rule_ids is not None and (
            not isinstance(observed_rule_ids, list)
            or any(not isinstance(rule_id, str) for rule_id in observed_rule_ids)
            or observed_rule_ids != sorted(set(observed_rule_ids))
        ):
            raise SandboxManifestError("mutation summary observed rule IDs are invalid")
        observation = item.get("rule_observation")
        if observation is not None and (
            not isinstance(observation, dict)
            or observation.get("status")
            not in {"matched", "failed", "not-declared"}
        ):
            raise SandboxManifestError("mutation summary rule observation is invalid")
        if item.get("status") not in {"passed", "failed", "unavailable"}:
            raise SandboxManifestError("mutation summary case has unsupported status")
        artifact_name = item.get("artifact")
        if not isinstance(artifact_name, str):
            raise SandboxManifestError("mutation summary artifact is invalid")
        artifact = (path.parent / artifact_name).resolve()
        try:
            artifact.relative_to(path.parent.resolve())
        except ValueError as error:
            raise SandboxManifestError(
                "mutation artifact path escapes summary directory"
            ) from error
        validated = validate_artifact(artifact, manifest_path)
        if validated["case_id"] != case_id:
            raise SandboxManifestError(
                "mutation artifact case does not match summary case"
            )
        if expected.expected_rule_ids:
            try:
                artifact_data = json.loads(artifact.read_text(encoding="utf-8"))
            except (OSError, ValueError) as error:
                raise SandboxManifestError(
                    f"cannot read mutation artifact {artifact}: {error}"
                ) from error
            from sandbox_mutation import _analysis, _rule_observation

            recomputed = _analysis(artifact_data)
            if item.get("observed_rule_ids") != recomputed["rule_ids"]:
                raise SandboxManifestError(
                    "mutation summary observed rule IDs do not match artifact"
                )
            summary_analysis = item.get("analysis")
            if (
                not isinstance(summary_analysis, dict)
                or summary_analysis.get("rule_ids") != recomputed["rule_ids"]
            ):
                raise SandboxManifestError(
                    "mutation summary analysis rule IDs do not match artifact"
                )
            expected_observation = _rule_observation(
                expected.expected_rule_ids,
                expected.mutation_kind,
                tuple(recomputed["rule_ids"]),
            )
            observation = item.get("rule_observation")
            if (
                not isinstance(observation, dict)
                or observation.get("status") != expected_observation[0]
                or observation.get("reason") != expected_observation[1]
            ):
                raise SandboxManifestError(
                    "mutation summary rule observation does not match artifact"
                )
        phases = item.get("phases")
        if not isinstance(phases, dict) or any(
            not isinstance(phases.get(name), dict)
            or phases[name].get("status") not in ALLOWED_STATUSES
            for name in ("baseline", "mutate", "oracle", "revert")
        ):
            raise SandboxManifestError("mutation summary phases are invalid")
        analysis = item.get("analysis")
        if analysis is not None:
            if not isinstance(analysis, dict) or analysis.get("status") not in {
                "measured",
                "unavailable",
            }:
                raise SandboxManifestError("mutation summary analysis is invalid")
            count = analysis.get("count")
            if count is not None and (
                not isinstance(count, int) or isinstance(count, bool) or count < 0
            ):
                raise SandboxManifestError("mutation summary analysis count is invalid")
            digest = analysis.get("sha256")
            if digest is not None and (
                not isinstance(digest, str) or not SHA256.fullmatch(digest)
            ):
                raise SandboxManifestError("mutation summary analysis hash is invalid")
            rule_ids = analysis.get("rule_ids", [])
            if (
                not isinstance(rule_ids, list)
                or any(not isinstance(rule_id, str) for rule_id in rule_ids)
                or rule_ids != sorted(set(rule_ids))
            ):
                raise SandboxManifestError("mutation summary analysis rule IDs are invalid")
    if seen != set(expected_cases):
        raise SandboxManifestError("mutation summary is missing manifest cases")
    return {"status": "valid", "cases": len(cases)}
