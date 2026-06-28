"""Reviewed precision expectations for the real-repo validation zoo.

Snapshots (`tests/zoo/snapshots/*.json`) answer *what RepoPilot emitted*.
Expectations (`tests/zoo/expectations/*.toml`) answer *how a human reviewed those
emissions*. This module is the pure parse + match + render layer: it has no
network access, no scanner knowledge, and never mutates analyzer output. It is
driven by `scripts/zoo.py` (live findings) and exercised directly by the Rust
`zoo_expectations_contract` integration test (synthetic findings).

Policy:
  * default profile is *exhaustive*  — every default-visible live finding must
    map to exactly one expectation, and every default expectation must map to
    exactly one live finding;
  * strict profile is *selective*    — listed strict expectations are recall
    anchors that must each still match exactly one live finding; unlisted strict
    findings are allowed.

Labels never suppress findings. `false-positive` is a reviewed disposition that
is reported prominently but does not fail the gate or hide the finding.
"""

from __future__ import annotations

import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1

DISPOSITIONS = ("actionable", "valid-but-accepted", "false-positive")
PROFILES = ("default", "strict")
COVERAGE = ("exhaustive", "selective")

# Optional, never-required metadata fields a reviewer may attach to an entry.
OPTIONAL_FINDING_KEYS = ("line", "line_end", "issue", "notes", "reviewed_by", "expires")
REQUIRED_FINDING_KEYS = ("profile", "finding_id", "rule_id", "path", "disposition", "reason")
ALLOWED_FINDING_KEYS = set(REQUIRED_FINDING_KEYS) | set(OPTIONAL_FINDING_KEYS)

# Diagnostic kinds. Every kind except KNOWN_FALSE_POSITIVE fails the zoo gate.
# SNAPSHOT_DRIFT lives in zoo.py (it compares emitted output, not labels) and is
# reported separately, but the label is defined here so renderers agree.
SNAPSHOT_DRIFT = "SNAPSHOT DRIFT"
UNLABELED_DEFAULT_FINDING = "UNLABELED DEFAULT FINDING"
STALE_EXPECTATION = "STALE EXPECTATION"
DUPLICATE_EXPECTATION = "DUPLICATE EXPECTATION"
INVALID_EXPECTATION = "INVALID EXPECTATION"
AMBIGUOUS_EXPECTATION = "AMBIGUOUS EXPECTATION"
MISSING_STRICT_RECALL_ANCHOR = "MISSING STRICT RECALL ANCHOR"
MISSING_EXPECTATION_FILE = "MISSING EXPECTATION FILE"
UNKNOWN_EXPECTATION_FILE = "UNKNOWN EXPECTATION FILE"
KNOWN_FALSE_POSITIVE = "KNOWN FALSE POSITIVE"

# Order used when grouping diagnostics for output (most urgent first).
DIAGNOSTIC_ORDER = (
    SNAPSHOT_DRIFT,
    UNLABELED_DEFAULT_FINDING,
    STALE_EXPECTATION,
    DUPLICATE_EXPECTATION,
    AMBIGUOUS_EXPECTATION,
    INVALID_EXPECTATION,
    MISSING_EXPECTATION_FILE,
    UNKNOWN_EXPECTATION_FILE,
    MISSING_STRICT_RECALL_ANCHOR,
    KNOWN_FALSE_POSITIVE,
)


def is_failure(kind: str) -> bool:
    """Every diagnostic except a reviewed known-false-positive fails the gate."""
    return kind != KNOWN_FALSE_POSITIVE


# ----------------------------------------------------------------------------
# Records
# ----------------------------------------------------------------------------
@dataclass(frozen=True)
class ExpectationFinding:
    profile: str
    finding_id: str
    rule_id: str
    path: str
    disposition: str
    reason: str
    line: int | None = None
    line_end: int | None = None
    issue: str | None = None
    notes: str | None = None
    reviewed_by: str | None = None
    expires: str | None = None
    index: int = 0  # 1-based position in file, for diagnostics

    def identity(self) -> tuple[str, str, str, str, int | None, int | None]:
        return (self.profile, self.finding_id, self.rule_id, self.path, self.line, self.line_end)


@dataclass(frozen=True)
class Expectation:
    repo: str
    schema_version: int
    default_coverage: str
    strict_coverage: str
    findings: tuple[ExpectationFinding, ...]
    source: str


@dataclass(frozen=True)
class LiveFinding:
    profile: str
    id: str
    rule_id: str
    path: str
    line: int | None
    line_end: int | None
    severity: str
    confidence: str
    priority: str
    title: str
    snippet: str


@dataclass(frozen=True)
class Diagnostic:
    kind: str
    repo: str
    message: str


@dataclass
class RepoReview:
    """Aggregated, snapshot-free review of one repo's labeled default findings."""

    repo: str
    default_total: int = 0
    labeled: int = 0
    unlabeled: int = 0
    actionable: int = 0
    valid_but_accepted: int = 0
    false_positive: int = 0
    strict_anchors: int = 0
    by_rule: dict[str, int] = field(default_factory=dict)


# ----------------------------------------------------------------------------
# Parse + structural validation
# ----------------------------------------------------------------------------
def parse_expectation_text(text: str, repo: str, source: str) -> tuple[Expectation | None, list[Diagnostic]]:
    """Parse + structurally validate one expectation file's TOML text.

    Returns (expectation_or_None, diagnostics). When the file is unparseable or
    its header is invalid the expectation is None and matching must be skipped;
    individual malformed `[[finding]]` entries are dropped with an INVALID
    diagnostic while the remaining valid entries are kept.
    """
    diags: list[Diagnostic] = []
    try:
        data = tomllib.loads(text)
    except tomllib.TOMLDecodeError as exc:
        return None, [Diagnostic(INVALID_EXPECTATION, repo, f"{source}: TOML parse error: {exc}")]

    schema = data.get("schema_version")
    if schema != SCHEMA_VERSION:
        diags.append(
            Diagnostic(
                INVALID_EXPECTATION,
                repo,
                f"{source}: unsupported schema_version {schema!r} (expected {SCHEMA_VERSION})",
            )
        )
        return None, diags

    declared_repo = data.get("repo")
    if declared_repo != repo:
        diags.append(
            Diagnostic(
                INVALID_EXPECTATION,
                repo,
                f"{source}: repo field {declared_repo!r} does not match filename repo {repo!r}",
            )
        )
        return None, diags

    default_coverage = data.get("default_coverage", "exhaustive")
    strict_coverage = data.get("strict_coverage", "selective")
    for label, value in (("default_coverage", default_coverage), ("strict_coverage", strict_coverage)):
        if value not in COVERAGE:
            diags.append(Diagnostic(INVALID_EXPECTATION, repo, f"{source}: {label} {value!r} not in {COVERAGE}"))
            return None, diags

    raw_findings = data.get("finding", [])
    if not isinstance(raw_findings, list):
        return None, [Diagnostic(INVALID_EXPECTATION, repo, f"{source}: [[finding]] must be an array of tables")]

    findings: list[ExpectationFinding] = []
    seen: dict[tuple, int] = {}
    for position, raw in enumerate(raw_findings, start=1):
        parsed, entry_diags = _parse_finding(raw, repo, source, position)
        diags.extend(entry_diags)
        if parsed is None:
            continue
        key = parsed.identity()
        if key in seen:
            diags.append(
                Diagnostic(
                    DUPLICATE_EXPECTATION,
                    repo,
                    f"{source}: entry #{position} duplicates entry #{seen[key]} ({parsed.finding_id} @ {parsed.path})",
                )
            )
            continue
        seen[key] = position
        findings.append(parsed)

    expectation = Expectation(
        repo=repo,
        schema_version=SCHEMA_VERSION,
        default_coverage=default_coverage,
        strict_coverage=strict_coverage,
        findings=tuple(findings),
        source=source,
    )
    return expectation, diags


def coverage_diagnostics(
    required_names: list[str], manifest_names: list[str], present_stems: list[str]
) -> list[Diagnostic]:
    """Every required repo must have a file; every file must map to a manifest repo."""
    diags: list[Diagnostic] = []
    present = set(present_stems)
    manifest = set(manifest_names)
    for name in required_names:
        if name not in present:
            diags.append(Diagnostic(MISSING_EXPECTATION_FILE, name, f"no {name}.toml in expectations dir"))
    for stem in sorted(present):
        if stem not in manifest:
            diags.append(Diagnostic(UNKNOWN_EXPECTATION_FILE, stem, f"{stem}.toml has no matching manifest [[repo]]"))
    return diags


def parse_expectation_file(path: Path, repo: str) -> tuple[Expectation | None, list[Diagnostic]]:
    source = path.name
    if not path.exists():
        return None, [Diagnostic(MISSING_EXPECTATION_FILE, repo, f"no expectation file at {source}")]
    return parse_expectation_text(path.read_text(encoding="utf-8"), repo, source)


def _parse_finding(
    raw: Any, repo: str, source: str, position: int
) -> tuple[ExpectationFinding | None, list[Diagnostic]]:
    where = f"{source}: entry #{position}"
    if not isinstance(raw, dict):
        return None, [Diagnostic(INVALID_EXPECTATION, repo, f"{where} is not a table")]

    unknown = set(raw) - ALLOWED_FINDING_KEYS
    if unknown:
        return None, [Diagnostic(INVALID_EXPECTATION, repo, f"{where} has unknown field(s): {', '.join(sorted(unknown))}")]

    def fail(msg: str) -> tuple[None, list[Diagnostic]]:
        return None, [Diagnostic(INVALID_EXPECTATION, repo, f"{where} {msg}")]

    profile = raw.get("profile")
    if profile not in PROFILES:
        return fail(f"has unknown profile {profile!r} (expected one of {PROFILES})")

    for key in ("finding_id", "rule_id", "path"):
        value = raw.get(key)
        if not isinstance(value, str) or not value.strip():
            return fail(f"is missing a non-empty {key}")

    disposition = raw.get("disposition")
    if disposition not in DISPOSITIONS:
        return fail(f"has unknown disposition {disposition!r} (expected one of {DISPOSITIONS})")

    reason = raw.get("reason")
    if not isinstance(reason, str) or not reason.strip():
        return fail("has an empty reason")

    line = raw.get("line")
    line_end = raw.get("line_end")
    for key, value in (("line", line), ("line_end", line_end)):
        if value is not None and (not isinstance(value, int) or isinstance(value, bool)):
            return fail(f"has a non-integer {key} {value!r}")

    return (
        ExpectationFinding(
            profile=profile,
            finding_id=raw["finding_id"],
            rule_id=raw["rule_id"],
            path=raw["path"],
            disposition=disposition,
            reason=reason,
            line=line,
            line_end=line_end,
            issue=raw.get("issue"),
            notes=raw.get("notes"),
            reviewed_by=raw.get("reviewed_by"),
            expires=raw.get("expires"),
            index=position,
        ),
        [],
    )


# ----------------------------------------------------------------------------
# Live findings
# ----------------------------------------------------------------------------
def parse_live_findings(report: dict[str, Any], clone_dir: str | Path, profile: str) -> list[LiveFinding]:
    raw = report.get("findings")
    findings = [f for f in raw if isinstance(f, dict)] if isinstance(raw, list) else []
    return [_live_from_dict(f, clone_dir, profile) for f in findings]


def _live_from_dict(finding: dict[str, Any], clone_dir: str | Path, profile: str) -> LiveFinding:
    evidence = finding.get("evidence")
    first = evidence[0] if isinstance(evidence, list) and evidence and isinstance(evidence[0], dict) else {}
    risk = finding.get("risk")
    priority = risk.get("priority") if isinstance(risk, dict) else None
    return LiveFinding(
        profile=profile,
        id=str(finding.get("id", "")),
        rule_id=str(finding.get("rule_id", "")),
        path=relative_path(first.get("path", ""), clone_dir),
        line=first.get("line_start") if isinstance(first.get("line_start"), int) else None,
        line_end=first.get("line_end") if isinstance(first.get("line_end"), int) else None,
        severity=str(finding.get("severity", "")),
        confidence=str(finding.get("confidence", "")),
        priority=str(priority) if isinstance(priority, str) else "unknown",
        title=str(finding.get("title", "")),
        snippet=str(first.get("snippet", "")),
    )


def relative_path(path: str, clone_dir: str | Path) -> str:
    """Normalize an evidence path to a clone-relative path.

    File-based findings carry an absolute path under the clone dir; graph
    findings (cycles) already carry a relative path. Both must collapse to the
    same machine-independent relative form the finding `id` encodes.
    """
    text = str(path)
    root = str(clone_dir).rstrip("/")
    if root and text.startswith(root + "/"):
        text = text[len(root) + 1 :]
    return text.lstrip("./") if text.startswith("./") else text


# ----------------------------------------------------------------------------
# Matching
# ----------------------------------------------------------------------------
def _matches(exp: ExpectationFinding, live: LiveFinding) -> bool:
    if exp.finding_id != live.id or exp.rule_id != live.rule_id or exp.path != live.path:
        return False
    if exp.line is not None and exp.line != live.line:
        return False
    if exp.line_end is not None and exp.line_end != live.line_end:
        return False
    return True


def match_repo(
    repo: str,
    expectation: Expectation,
    default_live: list[LiveFinding],
    strict_live: list[LiveFinding],
) -> tuple[list[Diagnostic], RepoReview]:
    """Match one repo's expectations against its live default + strict findings."""
    diags: list[Diagnostic] = []
    review = RepoReview(repo=repo, default_total=len(default_live))

    default_exps = [f for f in expectation.findings if f.profile == "default"]
    strict_exps = [f for f in expectation.findings if f.profile == "strict"]
    review.strict_anchors = len(strict_exps)

    diags += _match_profile(repo, default_exps, default_live, "default", exhaustive=True, review=review)
    diags += _match_profile(repo, strict_exps, strict_live, "strict", exhaustive=False, review=review)
    return diags, review


def _match_profile(
    repo: str,
    exps: list[ExpectationFinding],
    live: list[LiveFinding],
    profile: str,
    exhaustive: bool,
    review: RepoReview,
) -> list[Diagnostic]:
    diags: list[Diagnostic] = []
    exp_matches = {id(e): [lf for lf in live if _matches(e, lf)] for e in exps}
    live_matches = {idx: [e for e in exps if _matches(e, lf)] for idx, lf in enumerate(live)}

    for exp in exps:
        matched = exp_matches[id(exp)]
        if len(matched) == 0:
            kind = STALE_EXPECTATION if exhaustive else MISSING_STRICT_RECALL_ANCHOR
            diags.append(Diagnostic(kind, repo, _describe_exp(exp)))
        elif len(matched) > 1:
            diags.append(
                Diagnostic(
                    AMBIGUOUS_EXPECTATION,
                    repo,
                    f"{profile} expectation matches {len(matched)} live findings; add `line` to disambiguate: {_describe_exp(exp)}",
                )
            )
        else:
            _count_disposition(exp, matched[0], repo, diags, review)

    for idx, lf in enumerate(live):
        matched = live_matches[idx]
        if len(matched) > 1:
            diags.append(
                Diagnostic(
                    DUPLICATE_EXPECTATION,
                    repo,
                    f"{len(matched)} {profile} expectations match one live finding: {_describe_live(lf)}",
                )
            )
        elif len(matched) == 0 and exhaustive:
            diags.append(Diagnostic(UNLABELED_DEFAULT_FINDING, repo, _describe_live(lf)))

    return diags


def _count_disposition(
    exp: ExpectationFinding, live: LiveFinding, repo: str, diags: list[Diagnostic], review: RepoReview
) -> None:
    if exp.profile == "default":
        review.labeled += 1
        review.by_rule[exp.rule_id] = review.by_rule.get(exp.rule_id, 0) + 1
        if exp.disposition == "actionable":
            review.actionable += 1
        elif exp.disposition == "valid-but-accepted":
            review.valid_but_accepted += 1
        elif exp.disposition == "false-positive":
            review.false_positive += 1
    if exp.disposition == "false-positive":
        diags.append(
            Diagnostic(
                KNOWN_FALSE_POSITIVE,
                repo,
                f"{exp.profile} {_describe_live(live)} — reviewed false positive: {exp.reason}",
            )
        )


def _describe_exp(exp: ExpectationFinding) -> str:
    suffix = f" line {exp.line}" if exp.line is not None else ""
    return f"{exp.rule_id} @ {exp.path}{suffix} (id {exp.finding_id})"


def _describe_live(live: LiveFinding) -> str:
    suffix = f" line {live.line}" if live.line is not None else ""
    return f"{live.rule_id} @ {live.path}{suffix} (id {live.id})"


def unlabeled_default(expectation: Expectation | None, default_live: list[LiveFinding]) -> list[LiveFinding]:
    exps = [f for f in expectation.findings if f.profile == "default"] if expectation else []
    return [lf for lf in default_live if not any(_matches(e, lf) for e in exps)]
