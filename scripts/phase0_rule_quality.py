"""Validate the committed rule-quality evidence boundary for Phase 0."""

from __future__ import annotations

import hashlib
import json
import tomllib
from pathlib import Path
from typing import Any

import zoo_expectations as ze
import zoo_scorecard as zs

from phase0_evidence_model import Phase0Paths, base_track, path_label, short_error


PROTOCOL = "rule-quality-scorecard-v1"


class RuleQualityProtocolError(ValueError):
    """The committed rule-quality protocol inputs cannot be trusted."""


def _tree_hash(root: Path, paths: list[Path]) -> str:
    digest = hashlib.sha256()
    for path in sorted(paths):
        digest.update(str(path.relative_to(root)).encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def _manifest_repos(path: Path) -> list[dict[str, Any]]:
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise RuleQualityProtocolError(f"zoo manifest is unreadable: {error}") from error
    repos = data.get("repo")
    if not isinstance(repos, list) or not all(isinstance(repo, dict) for repo in repos):
        raise RuleQualityProtocolError("zoo manifest must contain [[repo]] tables")
    return repos


def _validate_snapshot_labels(
    repo: dict[str, Any], expectation_dir: Path, snapshot_dir: Path
) -> tuple[int, list[str], list[str]]:
    name = repo.get("name")
    if not isinstance(name, str) or not name:
        raise RuleQualityProtocolError("zoo manifest repo is missing a name")
    if "/" in name or "\\" in name or name in {".", ".."}:
        raise RuleQualityProtocolError(f"zoo manifest repo name is not a simple name: {name}")
    expectation_path = expectation_dir / f"{name}.toml"
    snapshot_path = snapshot_dir / f"{name}.json"
    if not expectation_path.is_file():
        raise FileNotFoundError(f"missing expectation file for {name}")
    if not snapshot_path.is_file():
        raise FileNotFoundError(f"missing snapshot for {name}")
    expectation, diagnostics = ze.parse_expectation_file(expectation_path, name)
    failures = [diagnostic.message for diagnostic in diagnostics if ze.is_failure(diagnostic.kind)]
    if expectation is None or failures:
        detail = "; ".join(failures) or "expectation file is invalid"
        raise ValueError(f"{name}: {detail}")
    if expectation.default_coverage != "exhaustive":
        raise ValueError(f"{name}: default coverage must be exhaustive")
    snapshot = json.loads(snapshot_path.read_text(encoding="utf-8"))
    default = snapshot.get("default")
    visible_total = default.get("visible_total") if isinstance(default, dict) else None
    if not isinstance(visible_total, int) or visible_total < 0:
        raise ValueError(f"{name}: snapshot default.visible_total is invalid")
    labels = [finding for finding in expectation.findings if finding.profile == "default"]
    if len(labels) != visible_total:
        raise ValueError(
            f"{name}: {len(labels)} labeled default findings do not match "
            f"snapshot total {visible_total}"
        )
    return (
        visible_total,
        [finding.rule_id for finding in labels],
        [finding.evidence for finding in expectation.findings],
    )


def _load_scorecard_data(
    paths: Phase0Paths,
    repos: list[dict[str, Any]],
    expectation_dir: Path,
    scorecard: Path,
) -> tuple[str, dict[str, str], dict[str, zs.RuleScore], dict[str, zs.RuleScore]]:
    if not scorecard.is_file():
        raise RuleQualityProtocolError("committed rule scorecard is missing")
    try:
        generated = zs.generate_scorecard(repos, expectation_dir, paths.rules_reference)
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise RuleQualityProtocolError(f"scorecard sources are invalid: {error}") from error
    committed = scorecard.read_text(encoding="utf-8")
    if committed != generated:
        raise RuleQualityProtocolError("committed rule scorecard is stale")
    lifecycles = zs.load_rule_lifecycles(paths.rules_reference)
    scores = zs.aggregate_rule_scores(repos, expectation_dir, "default")
    strict_scores = zs.aggregate_rule_scores(repos, expectation_dir, "strict", evidence="sample")
    return committed, lifecycles, scores, strict_scores


def _collect_snapshot_totals(
    repos: list[dict[str, Any]], expectation_dir: Path, snapshot_dir: Path
) -> tuple[int, list[str], list[str]]:
    snapshot_total = 0
    default_rule_ids: list[str] = []
    evidence_kinds: list[str] = []
    for repo in repos:
        total, rule_ids, kinds = _validate_snapshot_labels(repo, expectation_dir, snapshot_dir)
        snapshot_total += total
        default_rule_ids.extend(rule_ids)
        evidence_kinds.extend(kinds)
    return snapshot_total, default_rule_ids, evidence_kinds


def _quality_observation(
    lifecycles: dict[str, str],
    scores: dict[str, zs.RuleScore],
    strict_scores: dict[str, zs.RuleScore],
    snapshot_total: int,
    repo_count: int,
) -> dict[str, Any]:
    all_rule_ids = set(lifecycles) | set(scores)
    measured_rule_ids = {rule_id for rule_id, score in scores.items() if score.labeled > 0}
    unmeasured_rule_ids = sorted(all_rule_ids - measured_rule_ids)
    labeled_default = sum(score.labeled for score in scores.values())
    if labeled_default != snapshot_total:
        raise ValueError(
            f"scorecard labels {labeled_default} findings but snapshots contain {snapshot_total}"
        )
    total_rules = len(all_rule_ids)
    return {
        "rules_total": total_rules,
        "default_rules_measured": len(measured_rule_ids),
        "default_rules_unmeasured": len(unmeasured_rule_ids),
        "labeled_default_findings": labeled_default,
        "strict_sampled_rules": sum(score.labeled > 0 for score in strict_scores.values()),
        "strict_sampled_findings": sum(score.labeled for score in strict_scores.values()),
        "snapshot_repositories": repo_count,
        "default_coverage_ratio": (
            round(len(measured_rule_ids) / total_rules, 6) if total_rules else 0.0
        ),
        "unmeasured_rule_ids": unmeasured_rule_ids,
    }


def _quality_hashes(
    root: Path,
    paths: Phase0Paths,
    committed: str,
    expectation_dir: Path,
    snapshot_dir: Path,
) -> dict[str, str]:
    return {
        "scorecard": hashlib.sha256(committed.encode("utf-8")).hexdigest(),
        "rules_reference": hashlib.sha256(paths.rules_reference.read_bytes()).hexdigest(),
        "zoo_manifest": hashlib.sha256(paths.zoo_manifest.read_bytes()).hexdigest(),
        "expectations": _tree_hash(root, sorted(expectation_dir.glob("*.toml"))),
        "snapshots": _tree_hash(root, sorted(snapshot_dir.glob("*.json"))),
    }


def inspect_rule_quality(paths: Phase0Paths) -> dict[str, Any]:
    """Return deterministic scorecard coverage and fail-closed diagnostics."""
    root = paths.root
    expectation_dir = paths.zoo_expectation_dir or root / "tests/zoo/expectations"
    snapshot_dir = paths.zoo_snapshot_dir or root / "tests/zoo/snapshots"
    scorecard = paths.rule_scorecard or root / "docs/engineering/rule-scorecard.md"
    repos = _manifest_repos(paths.zoo_manifest)
    committed, lifecycles, scores, strict_scores = _load_scorecard_data(
        paths, repos, expectation_dir, scorecard
    )
    snapshot_total, default_rule_ids, evidence_kinds = _collect_snapshot_totals(
        repos, expectation_dir, snapshot_dir
    )
    observation = _quality_observation(
        lifecycles, scores, strict_scores, snapshot_total, len(repos)
    )
    unmeasured_rule_ids = observation["unmeasured_rule_ids"]
    total_rules = observation["rules_total"]
    return {
        "observation": observation,
        "hashes": _quality_hashes(root, paths, committed, expectation_dir, snapshot_dir),
        "evidence_status": "pending-labels",
        "label_state": "committed-reviewed-labels",
        "scope": "committed-reviewed-labels",
        "reason": (
            f"{len(unmeasured_rule_ids)} of {total_rules} registered rules have no "
            "default-profile zoo evidence"
            if unmeasured_rule_ids
            else "held-out recall and independent rule-quality review are still missing"
        ),
        "next_action": (
            "collect fresh labeled default findings and held-out recall cases for "
            "the unmeasured rules"
            if unmeasured_rule_ids
            else "complete held-out recall and independent rule-quality review"
        ),
        "claim_boundary": (
            "Committed labels describe the pinned zoo only; they do not establish "
            "production precision or recall."
        ),
        "evidence_kinds": sorted(set(evidence_kinds)),
        "default_rule_ids": sorted(set(default_rule_ids)),
    }


def rule_quality_track(paths: Phase0Paths) -> dict[str, Any]:
    track = base_track("rule-quality")
    track.update(
        protocol={"name": PROTOCOL, "source": "committed-zoo-scorecard"},
        limitations=[
            "Committed snapshots and labels are a pinned evidence baseline, not a fresh zoo rescan.",
            "Unmeasured rules are not clean, and labels do not establish recall.",
        ],
    )
    try:
        result = inspect_rule_quality(paths)
    except RuleQualityProtocolError as error:
        track.update(
            protocol_status="invalid",
            evidence_status="invalid",
            label_state="not-applicable",
            scope="none",
            reason=short_error("rule-quality evidence", error),
            next_action="repair the scorecard, snapshots, or expectation denominators and rerun this audit",
        )
        return track
    except FileNotFoundError as error:
        track.update(
            evidence_status="artifact-missing",
            label_state="not-supplied",
            scope="protocol-only",
            reason=short_error("rule-quality evidence", error),
            next_action="supply the committed scorecard, snapshots, and expectation files and rerun this audit",
        )
        return track
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        track.update(
            evidence_status="invalid",
            label_state="not-applicable",
            scope="none",
            reason=short_error("rule-quality evidence", error),
            next_action="repair the scorecard, snapshots, or expectation denominators and rerun this audit",
        )
        return track
    track.update(result)
    scorecard = paths.rule_scorecard or paths.root / "docs/engineering/rule-scorecard.md"
    track["artifact"] = path_label(scorecard, paths.root)
    return track
