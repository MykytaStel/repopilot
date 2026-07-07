from __future__ import annotations

import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import zoo_scorecard as zs  # noqa: E402  (after the sys.path fix above)

RULES_REFERENCE_FIXTURE = textwrap.dedent(
    """\
    # RepoPilot Rules Reference

    ## Architecture

    ### `architecture.circular-dependency` — Circular import dependency detected

    - **Severity:** HIGH
    - **Confidence:** HIGH
    - **Lifecycle:** stable
    - **Signal source:** import-graph

    Two or more files import each other, forming a cycle.

    ### `architecture.dead-module` — Dead module detected

    - **Severity:** LOW
    - **Confidence:** HIGH
    - **Lifecycle:** experimental
    - **Signal source:** import-graph

    This production file is not imported by any other project file.
    """
)


def expectation_toml(repo: str, findings: list[tuple[str, str, str, str]]) -> str:
    """Build minimal expectation TOML text: (profile, rule_id, disposition, path)."""
    lines = [
        "schema_version = 1",
        f'repo = "{repo}"',
        'default_coverage = "exhaustive"',
        'strict_coverage = "selective"',
        "",
    ]
    for profile, rule_id, disposition, path in findings:
        lines += [
            "[[finding]]",
            f'profile = "{profile}"',
            f'finding_id = "{rule_id}:{path}:deadbeef"',
            f'rule_id = "{rule_id}"',
            f'path = "{path}"',
            f'disposition = "{disposition}"',
            'reason = "fixture reason"',
            "",
        ]
    return "\n".join(lines)


class LoadRuleLifecyclesTests(unittest.TestCase):
    def test_parses_lifecycle_per_rule(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "rules-reference.md"
            path.write_text(RULES_REFERENCE_FIXTURE, encoding="utf-8")
            lifecycles = zs.load_rule_lifecycles(path)
        self.assertEqual(
            lifecycles,
            {
                "architecture.circular-dependency": "stable",
                "architecture.dead-module": "experimental",
            },
        )


class AggregateRuleScoresTests(unittest.TestCase):
    def test_aggregates_default_findings_across_repos_and_ignores_strict(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            expectation_dir = Path(tmp)
            (expectation_dir / "repo-a.toml").write_text(
                expectation_toml(
                    "repo-a",
                    [
                        ("default", "architecture.circular-dependency", "actionable", "a.py"),
                        ("default", "architecture.circular-dependency", "valid-but-accepted", "b.py"),
                        ("strict", "architecture.dead-module", "actionable", "c.py"),
                    ],
                ),
                encoding="utf-8",
            )
            (expectation_dir / "repo-b.toml").write_text(
                expectation_toml(
                    "repo-b",
                    [("default", "architecture.circular-dependency", "false-positive", "d.py")],
                ),
                encoding="utf-8",
            )
            manifest = [{"name": "repo-a"}, {"name": "repo-b"}]
            scores = zs.aggregate_rule_scores(manifest, expectation_dir)

        self.assertNotIn("architecture.dead-module", scores, "strict-profile findings must not be counted")
        score = scores["architecture.circular-dependency"]
        self.assertEqual(score.labeled, 3)
        self.assertEqual(score.actionable, 1)
        self.assertEqual(score.valid_but_accepted, 1)
        self.assertEqual(score.false_positive, 1)
        self.assertEqual(score.repos, {"repo-a", "repo-b"})
        self.assertAlmostEqual(score.precision_estimate, 2 / 3)

    def test_missing_expectation_file_is_skipped_not_raised(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            scores = zs.aggregate_rule_scores([{"name": "no-such-repo"}], Path(tmp))
        self.assertEqual(scores, {})


class RenderScorecardMarkdownTests(unittest.TestCase):
    def test_no_zoo_evidence_row_and_precision_formatting(self) -> None:
        lifecycles = {
            "architecture.circular-dependency": "stable",
            "architecture.dead-module": "experimental",
        }
        score = zs.RuleScore(
            rule_id="architecture.circular-dependency",
            labeled=4,
            actionable=3,
            valid_but_accepted=0,
            false_positive=1,
            repos={"repo-a"},
        )
        rendered = zs.render_scorecard_markdown({"architecture.circular-dependency": score}, lifecycles)

        self.assertIn("<!-- @generated", rendered)
        self.assertIn(
            "| `architecture.circular-dependency` | stable | 4 labeled across 1 repo(s) | 0.75 | 1 |",
            rendered,
        )
        self.assertIn("| `architecture.dead-module` | experimental | no zoo evidence | n/a | 0 |", rendered)

    def test_rule_with_no_labeled_findings_reports_zero_debt(self) -> None:
        score = zs.RuleScore(rule_id="x.rule", labeled=0)
        rendered = zs.render_scorecard_markdown(
            {"x.rule": score}, {"x.rule": "preview"}
        )
        self.assertIn("| `x.rule` | preview | no zoo evidence | n/a | 0 |", rendered)

    def test_output_is_deterministic_and_sorted(self) -> None:
        lifecycles = {"z.rule": "stable", "a.rule": "preview"}
        first = zs.render_scorecard_markdown({}, lifecycles)
        second = zs.render_scorecard_markdown({}, lifecycles)
        self.assertEqual(first, second)
        self.assertLess(first.index("`a.rule`"), first.index("`z.rule`"))


if __name__ == "__main__":
    unittest.main()
