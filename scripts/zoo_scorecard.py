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
from pathlib import Path
from typing import Any

import zoo_expectations as ze
from zoo_scorecard_stats import (
    MIN_DESCRIPTIVE_LABELS,
    RuleScore,
    format_rate,
    format_validity,
    wilson_interval,
)

GENERATED_BANNER = (
    "<!-- @generated from tests/zoo/expectations/*.toml and docs/rules-reference.md "
    "— do not edit by hand. -->\n"
    "<!-- Regenerate with `python3 scripts/zoo.py scorecard --write`. -->"
)

_RULE_HEADING = re.compile(r"^### `(?P<rule_id>[^`]+)`", re.MULTILINE)
_LIFECYCLE_LINE = re.compile(r"^- \*\*Lifecycle:\*\* (?P<lifecycle>\S+)", re.MULTILINE)
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


def aggregate_rule_scores(
    manifest: list[dict[str, Any]],
    expectation_dir: Path,
    profile: str = "default",
    evidence: str | None = None,
) -> dict[str, RuleScore]:
    """Aggregate every committed expectation file's findings for one profile.

    The two profiles are never mixed into one number: `default` labels are
    exhaustive per repo, while `strict` labels cover only what a reviewer chose
    to record, and averaging the two would present partial coverage as measured
    coverage. Passing `evidence` narrows a strict aggregate to one kind — only
    `sample` entries are drawn without looking at the finding, so only they can
    support a validity estimate.
    """
    scores: dict[str, RuleScore] = {}
    for repo in manifest:
        name = repo["name"]
        expectation, _diags = ze.parse_expectation_file(expectation_dir / f"{name}.toml", name)
        if expectation is None:
            continue
        for finding in expectation.findings:
            if finding.profile != profile:
                continue
            if evidence is not None and finding.evidence != evidence:
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


def render_strict_sample_section(scores: dict[str, RuleScore], lifecycles: dict[str, str]) -> list[str]:
    """The strict-profile table: sampled evidence, listed apart from the default one.

    Only rules a maintainer actually sampled appear here. A rule that fires
    thousands of times in strict is unmeasurable by exhaustive labeling, so a
    reproducible subset (`zoo.py sample --rule <id>`) is the only evidence it can
    earn — and it must never be read as coverage of the whole population.
    """
    lines = [
        "",
        "## Strict-profile sampled evidence",
        "",
        "Rules that fire only in the strict profile are too numerous to label "
        "exhaustively. These rows come from deterministic per-rule samples "
        "(`python3 scripts/zoo.py sample --rule <id>`), so the validity "
        "estimate describes the sampled findings, not the rule's full "
        "strict-profile population. A rule missing from this table has no "
        "sampled evidence at all.",
        "",
    ]
    sampled = {rule_id: score for rule_id, score in scores.items() if score.labeled > 0}
    if not sampled:
        lines.append("No rule has strict-profile sampled evidence yet.")
        lines.append("")
        return lines
    lines.append(
        "| Rule | Lifecycle | Sampled | Evidence Status | Validity (95% Wilson) | "
        "Actionability | False-Positive Rate | False-Positive Debt |"
    )
    lines.append("|---|---|---|---|---:|---:|---:|---:|")
    for rule_id in sorted(sampled):
        score = sampled[rule_id]
        lifecycle = lifecycles.get(rule_id, "unknown")
        sample = f"{score.labeled} sampled across {len(score.repos)} repo(s)"
        lines.append(
            f"| `{rule_id}` | {lifecycle} | {sample} | {score.evidence_status} | "
            f"{format_validity(score)} | {format_rate(score.actionability_estimate)} | "
            f"{format_rate(score.false_positive_rate)} | {score.false_positive} |"
        )
    lines.append("")
    return lines


def render_evidence_summary(
    scores: dict[str, RuleScore],
    lifecycles: dict[str, str],
    strict_scores: dict[str, RuleScore],
) -> list[str]:
    """Render scorecard denominators and the evidence coverage boundary."""
    rule_ids = set(lifecycles) | set(scores)
    measured = [score for score in scores.values() if score.labeled > 0]
    sampled = [score for score in strict_scores.values() if score.labeled > 0]
    labeled = sum(score.labeled for score in measured)
    repos = {repo for score in measured for repo in score.repos}
    strict_labeled = sum(score.labeled for score in sampled)
    strict_repos = {repo for score in sampled for repo in score.repos}
    total_rules = len(rule_ids)
    coverage = (len(measured) / total_rules * 100) if total_rules else 0.0
    return [
        "",
        "## Evidence coverage",
        "",
        f"- Default-profile evidence: {len(measured)} of {total_rules} rules "
        f"({coverage:.1f}%), {labeled} labeled findings across {len(repos)} repo(s).",
        f"- Default-profile rules without evidence: {total_rules - len(measured)} "
        "(unmeasured, not clean).",
        f"- Strict-profile sampled evidence: {len(sampled)} rules, {strict_labeled} "
        f"sampled findings across {len(strict_repos)} repo(s).",
        f"- Evidence status is `insufficient evidence` below {MIN_DESCRIPTIVE_LABELS} "
        "labeled findings; `descriptive` is a sample-size label, not a production precision claim.",
        "- These coverage counts describe committed labels and do not establish recall.",
        "",
    ]


def render_scorecard_markdown(
    scores: dict[str, RuleScore],
    lifecycles: dict[str, str],
    strict_scores: dict[str, RuleScore] | None = None,
) -> str:
    """Render the deterministic committed rule scorecard.

    One row per rule known to either source, so a rule with zero zoo evidence
    still appears (a coverage gap is itself useful signal) and a stale
    expectation referencing a since-renamed rule doesn't silently vanish.
    `strict_scores` adds the separate sampled-evidence table.
    """
    lines = [
        "# RepoPilot Rule Scorecard",
        "",
        GENERATED_BANNER,
        "",
        "Per-rule signal quality derived from the real-repo validation zoo "
        "(`tests/zoo/expectations/*.toml`) and each rule's lifecycle "
        "(`docs/rules-reference.md`). Validity estimate is "
        "`(actionable + valid-but-accepted) / labeled` default-profile zoo "
        "findings — a proxy from human-reviewed dispositions, not measured "
        "production precision. The validity column includes a 95% Wilson "
        "interval for the labeled sample; actionability and false-positive "
        "rate remain separate descriptive proportions. False-positive debt is the count of zoo findings a "
        "reviewer explicitly dispositioned `false-positive`; labels never "
        "suppress the finding, so debt reflects outstanding calibration work, "
        "not detector correctness at large.",
        "",
        "## Default-profile evidence",
        "",
        "Every default-visible zoo finding is labeled, so these rows are "
        "exhaustive for the pinned repositories. `no zoo evidence` means the "
        "rule never fired in the default profile on any of them — the rule is "
        "unmeasured, which is not the same as clean.",
        "",
        "| Rule | Lifecycle | Zoo Evidence | Evidence Status | Validity (95% Wilson) | "
        "Actionability | False-Positive Rate | False-Positive Debt |",
        "|---|---|---|---|---:|---:|---:|---:|",
    ]
    for rule_id in sorted(set(lifecycles) | set(scores)):
        lifecycle = lifecycles.get(rule_id, "unknown")
        score = scores.get(rule_id)
        if score is None or score.labeled == 0:
            evidence, status, validity, actionability, fp_rate, debt = (
                "no zoo evidence",
                "unmeasured",
                "n/a",
                "n/a",
                "n/a",
                "0",
            )
        else:
            evidence = f"{score.labeled} labeled across {len(score.repos)} repo(s)"
            status = score.evidence_status
            validity = format_validity(score)
            actionability = format_rate(score.actionability_estimate)
            fp_rate = format_rate(score.false_positive_rate)
            debt = str(score.false_positive)
        lines.append(
            f"| `{rule_id}` | {lifecycle} | {evidence} | {status} | {validity} | "
            f"{actionability} | {fp_rate} | {debt} |"
        )
    strict_scores = strict_scores or {}
    lines += render_evidence_summary(scores, lifecycles, strict_scores)
    lines += render_strict_sample_section(strict_scores, lifecycles)
    return "\n".join(lines)


def generate_scorecard(manifest: list[dict[str, Any]], expectation_dir: Path, rules_reference_path: Path) -> str:
    """Convenience wrapper: aggregate + parse lifecycles + render, in one call."""
    scores = aggregate_rule_scores(manifest, expectation_dir, "default")
    strict_scores = aggregate_rule_scores(manifest, expectation_dir, "strict", evidence="sample")
    lifecycles = load_rule_lifecycles(rules_reference_path)
    return render_scorecard_markdown(scores, lifecycles, strict_scores)
