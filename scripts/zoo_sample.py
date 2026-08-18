"""Deterministic per-rule sampling of zoo findings for precision measurement.

The expectation layer labels the *default* profile exhaustively, which works
because a healthy default profile is small. The strict profile is far too large
to label that way, so today most of the rule catalog carries no measured
evidence at all: a rule that only ever fires in strict can never earn a
precision estimate.

Sampling closes that gap. Given one rule, this module picks a fixed-size,
reproducible subset of that rule's findings across the whole zoo, which a
maintainer labels like any other expectation. The resulting numbers are a
*sample*, never exhaustive coverage, and every caller is expected to say so.

Two properties make the sample trustworthy:

  * **Deterministic** — findings are ordered by `(repo, path, line, id)` and the
    subset is chosen by even stride, so the same pinned zoo yields the same
    sample on every machine and every re-run.
  * **Spread** — even stride walks the whole ordered population instead of
    taking the alphabetically first N, so one large repository or one crowded
    directory cannot stand in for the rule's behavior everywhere.

Already-labeled findings are excluded, so re-running after a labeling pass
deepens coverage instead of re-offering the same entries.
"""

from __future__ import annotations

from dataclasses import dataclass

from zoo_expectations import LiveFinding


@dataclass(frozen=True)
class SampleCandidate:
    """One finding eligible for labeling, tagged with the repo it came from."""

    repo: str
    finding: LiveFinding

    @property
    def sort_key(self) -> tuple[str, str, int, str]:
        line = self.finding.line if self.finding.line is not None else -1
        return (self.repo, self.finding.path, line, self.finding.id)


@dataclass(frozen=True)
class RuleSample:
    """A reproducible subset of one rule's findings, plus its sampling frame."""

    rule_id: str
    profile: str
    population: int
    already_labeled: int
    repos_with_findings: tuple[str, ...]
    selected: tuple[SampleCandidate, ...]

    @property
    def is_exhaustive(self) -> bool:
        """True when the sample is the whole eligible population, not a subset."""
        return len(self.selected) == self.population


def collect_candidates(
    rule_id: str,
    live_by_repo: dict[str, list[LiveFinding]],
    labeled_ids_by_repo: dict[str, set[str]],
) -> list[SampleCandidate]:
    """Every unlabeled finding of `rule_id`, ordered deterministically.

    `labeled_ids_by_repo` carries the finding ids a repo's expectation file
    already dispositions, in any profile — re-labeling one is not useful work.
    """
    candidates = [
        SampleCandidate(repo=repo, finding=finding)
        for repo, findings in live_by_repo.items()
        for finding in findings
        if finding.rule_id == rule_id and finding.id not in labeled_ids_by_repo.get(repo, set())
    ]
    candidates.sort(key=lambda candidate: candidate.sort_key)
    return candidates


def even_stride(population: int, limit: int) -> list[int]:
    """Indices of `limit` items spread evenly across `population` items.

    The first and last items are always included when more than one is picked,
    so the sample spans the full ordered population rather than clustering at
    its start. Returns every index when the population is at or below `limit`.
    """
    if population <= 0 or limit <= 0:
        return []
    if population <= limit:
        return list(range(population))
    if limit == 1:
        return [0]
    step = (population - 1) / (limit - 1)
    return sorted({round(index * step) for index in range(limit)})


def build_sample(
    rule_id: str,
    profile: str,
    live_by_repo: dict[str, list[LiveFinding]],
    labeled_ids_by_repo: dict[str, set[str]],
    limit: int,
) -> RuleSample:
    candidates = collect_candidates(rule_id, live_by_repo, labeled_ids_by_repo)
    already_labeled = sum(
        1
        for repo, findings in live_by_repo.items()
        for finding in findings
        if finding.rule_id == rule_id and finding.id in labeled_ids_by_repo.get(repo, set())
    )
    repos = sorted({candidate.repo for candidate in candidates})
    selected = tuple(candidates[index] for index in even_stride(len(candidates), limit))
    return RuleSample(
        rule_id=rule_id,
        profile=profile,
        population=len(candidates),
        already_labeled=already_labeled,
        repos_with_findings=tuple(repos),
        selected=selected,
    )


def render_frame(sample: RuleSample) -> str:
    """The sampling frame, printed above the entries so labels stay honest.

    Anyone reading the resulting dispositions must be able to tell measured
    coverage from a subset without re-deriving it.
    """
    if sample.population == 0 and sample.already_labeled == 0:
        return f"rule `{sample.rule_id}` has no {sample.profile}-profile findings in the zoo"
    coverage = "exhaustive for the unlabeled population" if sample.is_exhaustive else "SAMPLE, not exhaustive"
    repos = ", ".join(sample.repos_with_findings) or "none"
    return "\n".join(
        [
            f"rule:        {sample.rule_id}",
            f"profile:     {sample.profile}",
            f"population:  {sample.population} unlabeled ({sample.already_labeled} already labeled)",
            f"repos:       {repos}",
            f"selected:    {len(sample.selected)} by even stride — {coverage}",
        ]
    )
