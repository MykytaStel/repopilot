from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from sandbox_report import render_report  # noqa: E402


class SandboxReportTests(unittest.TestCase):
    def test_pilot_report_explains_status_repeats_and_next_action(self) -> None:
        summary = {
            "kind": "pilot-summary",
            "status": "passed",
            "corpus": "technical-pilot-v1",
            "manifest_sha256": "a" * 64,
            "repeats": 2,
            "repeat_policy": ["cold", "warm"],
            "reason": "all cases have stable normalized output",
            "scanner": ["target/release/repopilot"],
            "cases": [
                {
                    "case_id": "control-case",
                    "project_id": "control",
                    "analysis_mode": "changed",
                    "status": "passed",
                    "comparison": {
                        "status": "stable",
                        "count": 3,
                        "normalized_sha256": "b" * 64,
                    },
                    "runs": [
                        {
                            "phase": "cold",
                            "status": "passed",
                            "oracle_status": "passed",
                            "normalized": {
                                "status": "measured",
                                "count": 3,
                            },
                        },
                        {
                            "phase": "warm",
                            "status": "passed",
                            "oracle_status": "passed",
                            "normalized": {
                                "status": "measured",
                                "count": 3,
                            },
                        },
                    ],
                }
            ],
        }

        report = render_report(summary, "markdown")

        self.assertIn("# Sandbox report", report)
        self.assertIn("**PASSED**", report)
        self.assertIn("2 runs per case: cold, warm", report)
        self.assertIn("stable; 3 normalized findings", report)
        self.assertIn("| `changed` |", report)
        self.assertIn("Next action", report)
        self.assertIn("technical reproducibility evidence", report)

    def test_mutation_report_marks_expected_oracle_failure_as_success(self) -> None:
        summary = {
            "kind": "mutation-summary",
            "status": "passed",
            "corpus": "mutation-packet-v1",
            "manifest_sha256": "a" * 64,
            "reason": "all mutation cases matched their expected oracle states",
            "cases": [
                {
                    "case_id": "broken-export",
                    "project_id": "express",
                    "mutation_kind": "violation",
                    "split": "evaluation",
                    "analysis_mode": "changed",
                    "status": "passed",
                    "expected_oracle": "failed",
                    "oracle_status": "failed",
                    "analysis": {
                        "status": "measured",
                        "count": 1,
                        "sha256": "c" * 64,
                    },
                    "phases": {
                        "baseline": {"status": "passed"},
                        "mutate": {"status": "passed"},
                        "oracle": {"status": "failed"},
                        "revert": {"status": "passed"},
                    },
                },
                {
                    "case_id": "unchanged-control",
                    "project_id": "express",
                    "mutation_kind": "negative-control",
                    "split": "evaluation",
                    "status": "passed",
                    "expected_oracle": "passed",
                    "oracle_status": "passed",
                    "phases": {
                        name: {"status": "passed"}
                        for name in ("baseline", "mutate", "oracle", "revert")
                    },
                },
            ],
        }

        report = render_report(summary, "markdown")

        self.assertIn("expected failure", report)
        self.assertIn("baseline `passed` → mutate `passed` → oracle `failed`", report)
        self.assertIn("negative control", report)
        self.assertIn("evaluation", report)
        self.assertIn("RepoPilot scan: 1 normalized finding", report)
        self.assertIn("| `changed` |", report)

    def test_mutation_report_flags_missing_violation_signal(self) -> None:
        summary = {
            "kind": "mutation-summary",
            "status": "passed",
            "corpus": "mutation-packet-v1",
            "manifest_sha256": "a" * 64,
            "reason": "all mutation cases matched their expected oracle states",
            "cases": [
                {
                    "case_id": "broken-export",
                    "project_id": "express",
                    "mutation_kind": "violation",
                    "split": "evaluation",
                    "status": "passed",
                    "expected_oracle": "failed",
                    "oracle_status": "failed",
                    "analysis": {"status": "measured", "count": 0},
                    "phases": {
                        name: {"status": "failed" if name == "oracle" else "passed"}
                        for name in ("baseline", "mutate", "oracle", "revert")
                    },
                }
            ],
        }

        report = render_report(summary, "markdown")

        self.assertIn("No RepoPilot findings were recorded", report)

    def test_report_redacts_sensitive_reason_text(self) -> None:
        summary = {
            "kind": "pilot-summary",
            "status": "unavailable",
            "corpus": "test",
            "manifest_sha256": "a" * 64,
            "repeats": 1,
            "repeat_policy": ["cold"],
            "reason": "scanner emitted secret-token-value",
            "cases": [],
        }

        report = render_report(summary, "text")

        self.assertNotIn("secret-token-value", report)
        self.assertIn("[REDACTED]", report)


if __name__ == "__main__":
    unittest.main()
