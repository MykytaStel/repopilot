"""Triage rendering for the zoo expectation layer.

CLI-facing rendering kept separate from parsing/matching (`zoo_expectations.py`):
it turns an unlabeled live finding into human-readable evidence plus a copy-paste
TOML skeleton. It NEVER decides a disposition — every generated entry carries the
non-valid placeholder `REVIEW_REQUIRED`, which expectation validation rejects, so
a maintainer must consciously classify each finding.
"""

from __future__ import annotations

from zoo_expectations import SCHEMA_VERSION, LiveFinding

REVIEW_REQUIRED = "REVIEW_REQUIRED"


def triage_skeleton(live: LiveFinding) -> str:
    lines = [
        "[[finding]]",
        f'profile = "{live.profile}"',
        f'finding_id = "{live.id}"',
        f'rule_id = "{live.rule_id}"',
        f'path = "{live.path}"',
    ]
    if live.line is not None:
        lines.append(f"line = {live.line}")
    lines.append(f'disposition = "{REVIEW_REQUIRED}"  # actionable | valid-but-accepted | false-positive')
    lines.append('reason = ""')
    return "\n".join(lines)


def render_triage_entry(repo: str, live: LiveFinding) -> str:
    line = f"{live.line}" if live.line is not None else "?"
    snippet = live.snippet.replace("\n", "\\n")
    if len(snippet) > 160:
        snippet = snippet[:160] + "…"
    header = [
        f"repo:        {repo}",
        f"profile:     {live.profile}",
        f"finding_id:  {live.id}",
        f"rule_id:     {live.rule_id}",
        f"priority:    {live.priority}",
        f"severity:    {live.severity}",
        f"confidence:  {live.confidence}",
        f"path:line:   {live.path}:{line}",
        f"title:       {live.title}",
        f"evidence:    {snippet}",
    ]
    return "\n".join(header) + "\n\n" + triage_skeleton(live) + "\n"


def skeleton_file_text(repo: str, lives: list[LiveFinding]) -> str:
    head = [
        f"schema_version = {SCHEMA_VERSION}",
        f'repo = "{repo}"',
        'default_coverage = "exhaustive"',
        'strict_coverage = "selective"',
        "",
        "# Auto-generated REVIEW skeleton. Replace every REVIEW_REQUIRED disposition",
        "# and fill each reason before validation will pass.",
        "",
    ]
    blocks = [triage_skeleton(lf) for lf in lives]
    return "\n".join(head) + ("\n\n".join(blocks) + "\n" if blocks else "")
