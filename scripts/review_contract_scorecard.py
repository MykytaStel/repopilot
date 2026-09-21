#!/usr/bin/env python3
"""Validate and render the review-zoo contract evidence protocol.

This scorecard is intentionally separate from the real-repository rule
scorecard. It measures the committed safe/unsafe fixture matrix and the
contract expectations that the Rust review-zoo harness checks at runtime. It
does not run the scanner and cannot establish production precision or recall.
"""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURE_ROOT = REPO_ROOT / "tests" / "fixtures" / "review-zoo"
SCORECARD_PATH = REPO_ROOT / "docs" / "engineering" / "review-contract-evidence.md"

GENERATED_BANNER = (
    "<!-- @generated from tests/fixtures/review-zoo/**/expected.json and "
    "tests/review_zoo.rs — do not edit by hand. -->\n"
    "<!-- Regenerate with `python3 scripts/review_contract_scorecard.py --write`. -->"
)

CONTRACT_CHANGES: dict[str, set[str]] = {
    "security-boundary": {"boundary-changed", "entrypoint-impacted"},
    "test-coverage": {"test-changed", "test-missing"},
}
REQUIRED_CONTRACT_FIELDS = {
    "family",
    "change",
    "exporter_path",
    "consumer_path",
    "confidence",
}


class ScorecardError(ValueError):
    """Raised when the committed fixture protocol is incomplete or malformed."""


@dataclass
class ContractFamilyScore:
    family: str
    positive_fixtures: int = 0
    expectation_count: int = 0
    changes: set[str] = field(default_factory=set)
    scenarios: set[str] = field(default_factory=set)


@dataclass(frozen=True)
class FixtureRecord:
    family: str
    scenario: str
    variant: str
    description: str
    contract_expectations: tuple[dict[str, str], ...]


@dataclass
class ContractScorecard:
    fixtures: list[FixtureRecord]
    by_contract_family: dict[str, ContractFamilyScore]

    @property
    def scenario_keys(self) -> set[tuple[str, str]]:
        return {(fixture.family, fixture.scenario) for fixture in self.fixtures}

    @property
    def scenario_count(self) -> int:
        return len(self.scenario_keys)

    @property
    def safe_variant_count(self) -> int:
        return sum(fixture.variant == "safe" for fixture in self.fixtures)

    @property
    def unsafe_variant_count(self) -> int:
        return sum(fixture.variant == "unsafe" for fixture in self.fixtures)

    @property
    def contract_positive_fixture_count(self) -> int:
        return sum(bool(fixture.contract_expectations) for fixture in self.fixtures)

    @property
    def contract_expectation_count(self) -> int:
        return sum(len(fixture.contract_expectations) for fixture in self.fixtures)


def _read_expected(path: Path) -> dict[str, Any]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ScorecardError(f"{path}: invalid expected.json: {exc}") from exc
    if not isinstance(payload, dict):
        raise ScorecardError(f"{path}: expected.json must contain an object")
    description = payload.get("description")
    if not isinstance(description, str) or not description.strip():
        raise ScorecardError(f"{path}: description must be a non-empty string")
    return payload


def _contract_expectations(payload: dict[str, Any], path: Path, variant: str) -> tuple[dict[str, str], ...]:
    raw = payload.get("contract_expect", [])
    if not isinstance(raw, list):
        raise ScorecardError(f"{path}: contract_expect must be an array")
    if variant == "safe" and raw:
        raise ScorecardError(f"{path}: safe fixture must not declare contract_expect")

    normalized: list[dict[str, str]] = []
    seen: set[tuple[tuple[str, str], ...]] = set()
    for index, value in enumerate(raw):
        if not isinstance(value, dict):
            raise ScorecardError(f"{path}: contract_expect[{index}] must be an object")
        missing = REQUIRED_CONTRACT_FIELDS - value.keys()
        if missing:
            names = ", ".join(sorted(missing))
            raise ScorecardError(f"{path}: contract_expect[{index}] missing {names}")
        if set(value) - REQUIRED_CONTRACT_FIELDS:
            unknown = ", ".join(sorted(set(value) - REQUIRED_CONTRACT_FIELDS))
            raise ScorecardError(f"{path}: contract_expect[{index}] has unknown fields {unknown}")
        if any(not isinstance(value[key], str) or not value[key].strip() for key in REQUIRED_CONTRACT_FIELDS):
            raise ScorecardError(f"{path}: contract_expect[{index}] fields must be non-empty strings")
        family = value["family"]
        change = value["change"]
        if family not in CONTRACT_CHANGES:
            raise ScorecardError(f"{path}: unknown contract family {family!r}")
        if change not in CONTRACT_CHANGES[family]:
            raise ScorecardError(f"{path}: unknown change {change!r} for {family!r}")
        if value["confidence"] != "limited":
            raise ScorecardError(f"{path}: contract evidence must remain limited confidence")
        identity = tuple(sorted(value.items()))
        if identity in seen:
            raise ScorecardError(f"{path}: duplicate contract expectation {family}/{change}")
        seen.add(identity)
        normalized.append({key: value[key] for key in sorted(value)})
    return tuple(normalized)


def collect_score(fixture_root: Path = FIXTURE_ROOT) -> ContractScorecard:
    """Validate every review-zoo pair and return deterministic contract counts."""
    if not fixture_root.is_dir():
        raise ScorecardError(f"fixture root not found: {fixture_root}")

    records: list[FixtureRecord] = []
    pairs: dict[tuple[str, str], set[str]] = defaultdict(set)
    for family_dir in sorted(path for path in fixture_root.iterdir() if path.is_dir()):
        for scenario_dir in sorted(path for path in family_dir.iterdir() if path.is_dir()):
            key = (family_dir.name, scenario_dir.name)
            for variant in ("safe", "unsafe"):
                variant_dir = scenario_dir / variant
                if not variant_dir.is_dir():
                    raise ScorecardError(f"{scenario_dir}: missing {variant} variant")
                expected_path = variant_dir / "expected.json"
                if not expected_path.is_file():
                    raise ScorecardError(f"{variant_dir}: missing expected.json")
                payload = _read_expected(expected_path)
                expectations = payload.get("expect", [])
                if not isinstance(expectations, list):
                    raise ScorecardError(f"{expected_path}: expect must be an array")
                if variant == "unsafe" and not expectations:
                    raise ScorecardError(f"{expected_path}: unsafe fixture must declare expect")
                if variant == "safe" and expectations:
                    raise ScorecardError(f"{expected_path}: safe fixture must not declare expect")
                contract_expectations = _contract_expectations(payload, expected_path, variant)
                pairs[key].add(variant)
                records.append(
                    FixtureRecord(
                        family=family_dir.name,
                        scenario=scenario_dir.name,
                        variant=variant,
                        description=payload["description"].strip(),
                        contract_expectations=contract_expectations,
                    )
                )

    if not records:
        raise ScorecardError(f"{fixture_root}: no fixture pairs found")
    for key, variants in pairs.items():
        if variants != {"safe", "unsafe"}:
            raise ScorecardError(f"{key[0]}/{key[1]}: missing safe variant or unsafe variant")

    by_family: dict[str, ContractFamilyScore] = {}
    for record in records:
        families_in_fixture: set[str] = set()
        for expectation in record.contract_expectations:
            family_score = by_family.setdefault(
                expectation["family"], ContractFamilyScore(expectation["family"])
            )
            if expectation["family"] not in families_in_fixture:
                family_score.positive_fixtures += 1
                families_in_fixture.add(expectation["family"])
            family_score.expectation_count += 1
            family_score.changes.add(expectation["change"])
            family_score.scenarios.add(f"{record.family}/{record.scenario}")
    missing_families = sorted(set(CONTRACT_CHANGES) - set(by_family))
    if missing_families:
        raise ScorecardError(
            "promoted contract families have no fixture evidence: "
            + ", ".join(missing_families)
        )
    return ContractScorecard(sorted(records, key=lambda item: (item.family, item.scenario, item.variant)), by_family)


def render_scorecard(score: ContractScorecard) -> str:
    """Render the committed, deterministic fixture evidence document."""
    lines = [
        "# Review Contract Evidence",
        "",
        GENERATED_BANNER,
        "",
        "This scorecard reports **synthetic fixture evidence** for the contract "
        "deltas emitted by the review pipeline. It is a protocol and coverage "
        "check: the Rust `review_zoo` test runs the real CLI against every pair "
        "and matches these expectations. It does not establish real-repository "
        "precision or recall.",
        "",
        "## Coverage",
        "",
        f"- Fixture pairs: {score.scenario_count} of {score.scenario_count} scenario pairs "
        f"have both safe and unsafe variants.",
        f"- Runtime controls: {score.safe_variant_count} safe variants are required to "
        "emit zero review signals and zero contract deltas.",
        f"- Contract-positive unsafe fixtures: {score.contract_positive_fixture_count} "
        f"of {score.unsafe_variant_count} unsafe variants; "
        f"{score.contract_expectation_count} contract expectations total.",
        "- All contract evidence is `limited` confidence; static paths do not prove "
        "runtime reachability, authorization behavior, or test execution coverage.",
        "- These counts describe committed fixtures and do not establish "
        "real-repository precision or recall.",
        "",
        "## Contract-family matrix",
        "",
        "| Contract family | Changes covered | Positive fixtures | Expectations | Scenario pairs |",
        "|---|---|---:|---:|---:|",
    ]
    for family in sorted(score.by_contract_family):
        family_score = score.by_contract_family[family]
        lines.append(
            f"| `{family}` | {', '.join(f'`{change}`' for change in sorted(family_score.changes))} | "
            f"{family_score.positive_fixtures} | {family_score.expectation_count} | "
            f"{len(family_score.scenarios)} |"
        )
    if not score.by_contract_family:
        lines.append("| _none_ | — | 0 | 0 | 0 |")

    lines += ["", "## Fixture matrix", "", "| Review family | Scenario | Unsafe contract evidence | Safe control |", "|---|---|---|---|"]
    for family, scenario in sorted(score.scenario_keys):
        unsafe = next(item for item in score.fixtures if item.family == family and item.scenario == scenario and item.variant == "unsafe")
        labels = sorted({f"{item['family']}/{item['change']}" for item in unsafe.contract_expectations})
        contract_label = ", ".join(f"`{label}`" for label in labels) if labels else "none"
        lines.append(f"| `{family}` | `{scenario}` | {contract_label} | zero contract deltas |")
    lines += [
        "",
        "## Reproduction",
        "",
        "```bash",
        "python3 scripts/review_contract_scorecard.py --check",
        "cargo test --test review_zoo",
        "```",
        "",
        "The scorecard validates fixture metadata; `cargo test --test review_zoo` is "
        "the execution evidence that the current binary satisfies it.",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="write the generated scorecard")
    parser.add_argument("--check", action="store_true", help="fail when the scorecard is stale")
    args = parser.parse_args()
    try:
        rendered = render_scorecard(collect_score())
    except ScorecardError as exc:
        parser.error(str(exc))
    if args.write:
        SCORECARD_PATH.write_text(rendered, encoding="utf-8")
        print(f"wrote {SCORECARD_PATH.relative_to(REPO_ROOT)}")
    elif args.check:
        if not SCORECARD_PATH.is_file() or SCORECARD_PATH.read_text(encoding="utf-8") != rendered:
            print(f"STALE {SCORECARD_PATH.relative_to(REPO_ROOT)}; run with --write")
            return 1
        print(f"fresh {SCORECARD_PATH.relative_to(REPO_ROOT)}")
    else:
        print(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
