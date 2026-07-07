"""Per-rule scorecard derived from the zoo's committed expectations.

Combines two already-committed, clone-free sources:
  * `tests/zoo/expectations/*.toml` — human-reviewed dispositions per finding,
    parsed via `zoo_expectations.parse_expectation_file` (no clones, no scan);
  * `docs/rules-reference.md` — the generated rule registry doc, which lists
    each rule's lifecycle.

This module has no network access and never runs the scanner; it is driven by
`scripts/zoo.py scorecard` and exercised directly by
`scripts/tests/test_zoo_scorecard.py`.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import zoo_expectations as ze

GENERATED_BANNER = (
    "<!-- @generated from tests/zoo/expectations/*.toml and docs/rules-reference.md "
    "— do not edit by hand. -->\n"
    "<!-- Regenerate with `python3 scripts/zoo.py scorecard --write`. -->"
)

_RULE_HEADING = re.compile(r"^### `(?P<rule_id>[^`]+)`", re.MULTILINE)
_LIFECYCLE_LINE = re.compile(r"^- \*\*Lifecycle:\*\* (?P<lifecycle>\S+)", re.MULTILINE)


@dataclass
class RuleScore:
    """Aggregated default-profile zoo evidence for one rule."""

    rule_id: str
    labeled: int = 0
    actionable: int = 0
    valid_but_accepted: int = 0
    false_positive: int = 0
    repos: set[str] = field(default_factory=set)

    @property
    def precision_estimate(self) -> float | None:
        """`(actionable + valid-but-accepted) / labeled`; `None` if never labeled.

        A proxy derived from reviewed dispositions, not measured precision.
        """
        if self.labeled == 0:
            return None
        return (self.actionable + self.valid_but_accepted) / self.labeled


def load_rule_lifecycles(rules_reference_path: Path) -> dict[str, str]:
    """Parse `rule_id -> lifecycle` out of the generated rules reference doc."""
    text = rules_reference_path.read_text(encoding="utf-8")
    headings = list(_RULE_HEADING.finditer(text))
    lifecycles: dict[str, str] = {}
    for index, match in enumerate(headings):
        start = match.end()
        end = headings[index + 1].start() if index + 1 < len(headings) else len(text)
        lifecycle_match = _LIFECYCLE_LINE.search(text[start:end])
        if lifecycle_match:
            lifecycles[match.group("rule_id")] = lifecycle_match.group("lifecycle")
    return lifecycles


def aggregate_rule_scores(manifest: list[dict[str, Any]], expectation_dir: Path) -> dict[str, RuleScore]:
    """Aggregate every committed expectation file's default-profile findings by rule_id."""
    scores: dict[str, RuleScore] = {}
    for repo in manifest:
        name = repo["name"]
        expectation, _diags = ze.parse_expectation_file(expectation_dir / f"{name}.toml", name)
        if expectation is None:
            continue
        for finding in expectation.findings:
            if finding.profile != "default":
                continue
            score = scores.setdefault(finding.rule_id, RuleScore(rule_id=finding.rule_id))
            score.labeled += 1
            score.repos.add(name)
            if finding.disposition == "actionable":
                score.actionable += 1
            elif finding.disposition == "valid-but-accepted":
                score.valid_but_accepted += 1
            elif finding.disposition == "false-positive":
                score.false_positive += 1
    return scores


def render_scorecard_markdown(scores: dict[str, RuleScore], lifecycles: dict[str, str]) -> str:
    """Render the deterministic committed rule scorecard.

    One row per rule known to either source, so a rule with zero zoo evidence
    still appears (a coverage gap is itself useful signal) and a stale
    expectation referencing a since-renamed rule doesn't silently vanish.
    """
    lines = [
        "# RepoPilot Rule Scorecard",
        "",
        GENERATED_BANNER,
        "",
        "Per-rule signal quality derived from the real-repo validation zoo "
        "(`tests/zoo/expectations/*.toml`) and each rule's lifecycle "
        "(`docs/rules-reference.md`). Precision estimate is "
        "`(actionable + valid-but-accepted) / labeled` default-profile zoo "
        "findings — a proxy from human-reviewed dispositions, not measured "
        "precision. False-positive debt is the count of zoo findings a "
        "reviewer explicitly dispositioned `false-positive`; labels never "
        "suppress the finding, so debt reflects outstanding calibration work, "
        "not detector correctness at large.",
        "",
        "| Rule | Lifecycle | Zoo Evidence | Precision Estimate | False-Positive Debt |",
        "|---|---|---|---:|---:|",
    ]
    for rule_id in sorted(set(lifecycles) | set(scores)):
        lifecycle = lifecycles.get(rule_id, "unknown")
        score = scores.get(rule_id)
        if score is None or score.labeled == 0:
            evidence, precision, debt = "no zoo evidence", "n/a", "0"
        else:
            evidence = f"{score.labeled} labeled across {len(score.repos)} repo(s)"
            precision = f"{score.precision_estimate:.2f}"
            debt = str(score.false_positive)
        lines.append(f"| `{rule_id}` | {lifecycle} | {evidence} | {precision} | {debt} |")
    lines.append("")
    return "\n".join(lines)


def generate_scorecard(manifest: list[dict[str, Any]], expectation_dir: Path, rules_reference_path: Path) -> str:
    """Convenience wrapper: aggregate + parse lifecycles + render, in one call."""
    scores = aggregate_rule_scores(manifest, expectation_dir)
    lifecycles = load_rule_lifecycles(rules_reference_path)
    return render_scorecard_markdown(scores, lifecycles)
