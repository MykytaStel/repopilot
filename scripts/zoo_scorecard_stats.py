"""Statistics and formatting helpers for the per-rule zoo scorecard."""

from __future__ import annotations

import math
from dataclasses import dataclass, field


MIN_DESCRIPTIVE_LABELS = 10
WILSON_Z = 1.96


def wilson_interval(successes: int, trials: int) -> tuple[float, float] | None:
    """Return a deterministic 95% Wilson interval for a bounded proportion.

    A missing interval means there were no labeled observations. The interval
    describes the reviewed sample only; it is not a production precision claim.
    """
    if trials < 0 or successes < 0 or successes > trials:
        raise ValueError("successes must be between zero and trials")
    if trials == 0:
        return None
    proportion = successes / trials
    z_squared = WILSON_Z**2
    denominator = 1 + z_squared / trials
    center = (proportion + z_squared / (2 * trials)) / denominator
    margin = (
        WILSON_Z
        * math.sqrt((proportion * (1 - proportion) + z_squared / (4 * trials)) / trials)
        / denominator
    )
    return max(0.0, center - margin), min(1.0, center + margin)


@dataclass
class RuleScore:
    """Aggregated zoo evidence for one rule and one profile."""

    rule_id: str
    labeled: int = 0
    actionable: int = 0
    valid_but_accepted: int = 0
    false_positive: int = 0
    repos: set[str] = field(default_factory=set)

    @property
    def precision_estimate(self) -> float | None:
        """Backward-compatible alias for the reviewed validity estimate."""
        if self.labeled == 0:
            return None
        return (self.actionable + self.valid_but_accepted) / self.labeled

    @property
    def validity_estimate(self) -> float | None:
        """Reviewed-valid proportion used by the scorecard."""
        return self.precision_estimate

    @property
    def actionability_estimate(self) -> float | None:
        if self.labeled == 0:
            return None
        return self.actionable / self.labeled

    @property
    def false_positive_rate(self) -> float | None:
        if self.labeled == 0:
            return None
        return self.false_positive / self.labeled

    @property
    def validity_interval(self) -> tuple[float, float] | None:
        return wilson_interval(self.actionable + self.valid_but_accepted, self.labeled)

    @property
    def evidence_status(self) -> str:
        if self.labeled == 0:
            return "unmeasured"
        if self.labeled < MIN_DESCRIPTIVE_LABELS:
            return "insufficient evidence"
        return "descriptive"


def format_rate(value: float | None) -> str:
    return "n/a" if value is None else f"{value:.2f}"


def format_validity(score: RuleScore | None) -> str:
    if score is None or score.validity_estimate is None:
        return "n/a"
    interval = score.validity_interval
    if interval is None:
        return "n/a"
    return f"{score.validity_estimate:.2f} ({interval[0]:.2f}–{interval[1]:.2f})"
