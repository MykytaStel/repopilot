from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from sandbox_mutation import _case_status  # noqa: E402


def artifact(*, baseline="passed", mutate="passed", oracle="passed", revert="passed"):
    return {
        "phases": [
            {"name": "baseline", "status": baseline},
            {"name": "mutate", "status": mutate},
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


if __name__ == "__main__":
    unittest.main()
