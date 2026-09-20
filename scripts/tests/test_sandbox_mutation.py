from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from sandbox_case import _redact_command  # noqa: E402
from sandbox_mutation import _analysis, _case_status, _rule_observation  # noqa: E402


def artifact(
    *, baseline="passed", mutate="passed", oracle="passed", revert="passed",
    analysis="measured", rule_ids=()
):
    return {
        "phases": [
            {"name": "baseline", "status": baseline},
            {"name": "mutate", "status": mutate},
            {
                "name": "analyze",
                "status": "passed" if analysis == "measured" else analysis,
                "result": {"normalized_findings": {"status": analysis, "rule_ids": list(rule_ids)}},
            },
            {"name": "oracle", "status": oracle},
            {"name": "revert", "status": revert},
        ]
    }


class SandboxMutationTests(unittest.TestCase):
    def test_expected_failed_oracle_is_a_passing_violation_case(self) -> None:
        self.assertEqual(_case_status(artifact(oracle="failed"), "failed"), "passed")

    def test_expected_passed_oracle_is_a_passing_negative_control(self) -> None:
        self.assertEqual(_case_status(artifact(), "passed"), "passed")

    def test_unavailable_setup_does_not_count_as_pass(self) -> None:
        self.assertEqual(
            _case_status(artifact(baseline="unavailable"), "passed"),
            "unavailable",
        )

    def test_wrong_oracle_state_is_failed(self) -> None:
        self.assertEqual(_case_status(artifact(), "failed"), "failed")

    def test_violation_requires_declared_rule_signal(self) -> None:
        self.assertEqual(
            _rule_observation(("demo.rule",), "violation", ("demo.rule",)),
            ("matched", None),
        )
        self.assertEqual(
            _rule_observation(("demo.rule",), "violation", ()),
            ("failed", "missing expected rule id: demo.rule"),
        )

    def test_negative_control_rejects_declared_rule_signal(self) -> None:
        self.assertEqual(
            _rule_observation(("demo.rule",), "negative-control", ()),
            ("matched", None),
        )
        self.assertEqual(
            _rule_observation(("demo.rule",), "negative-control", ("demo.rule",)),
            ("failed", "negative control emitted expected rule id: demo.rule"),
        )

    def test_declared_rule_requires_measured_analysis(self) -> None:
        self.assertEqual(
            _case_status(
                artifact(analysis="unavailable"),
                "passed",
                expected_rule_ids=("demo.rule",),
                mutation_kind="violation",
            ),
            "unavailable",
        )

    def test_rule_observation_uses_new_finding_identities(self) -> None:
        result = _analysis(
            {
                "phases": [
                    {
                        "name": "analyze",
                        "status": "passed",
                        "result": {
                            "normalized_findings": {
                                "status": "measured",
                                "count": 2,
                                "findings": [
                                    {"rule_id": "demo.rule", "path": "old.py", "line": 1},
                                    {"rule_id": "demo.rule", "path": "new.py", "line": 2},
                                ],
                            },
                            "baseline_normalized_findings": {
                                "status": "measured",
                                "findings": [
                                    {"rule_id": "demo.rule", "path": "old.py", "line": 1}
                                ],
                            },
                        },
                    }
                ]
            }
        )

        self.assertEqual(result["rule_ids"], ["demo.rule"])
        self.assertEqual(result["all_rule_ids"], ["demo.rule"])

    def test_rule_observation_matches_evidence_when_line_numbers_shift(self) -> None:
        stable = {
            "rule_id": "demo.rule",
            "path": "src.py",
            "line": 20,
            "finding_id": "demo.rule:src.py:stable",
            "evidence_sha256": "evidence",
        }
        shifted = {**stable, "line": 21}
        result = _analysis(
            {
                "phases": [
                    {
                        "name": "analyze",
                        "status": "passed",
                        "result": {
                            "normalized_findings": {
                                "status": "measured",
                                "findings": [shifted],
                            },
                            "baseline_normalized_findings": {
                                "status": "measured",
                                "findings": [stable],
                            },
                        },
                    }
                ]
            }
        )

        self.assertEqual(result["rule_ids"], [])

    def test_eval_code_is_redacted_in_recorded_commands(self) -> None:
        self.assertEqual(
            _redact_command(("node", "-e", "contains-secret-token")),
            ["node", "[REDACTED]", "[REDACTED]"],
        )


if __name__ == "__main__":
    unittest.main()
