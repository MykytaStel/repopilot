from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from differential_coverage import render_coverage_audit  # noqa: E402


class DifferentialCoverageTests(unittest.TestCase):
    def test_render_keeps_unavailable_reasons_and_measurement_boundary(self) -> None:
        report = render_coverage_audit(
            {
                "corpus": "holdout",
                "protocol": "differential-utility-v1",
                "baseline_evidence": {
                    "tracked": 6,
                    "measured": 6,
                    "comparison": {
                        "measured": 3,
                        "unavailable": 3,
                        "untracked": 0,
                        "by_baseline": {
                            "python.compile": {
                                "total": 3,
                                "measured": 3,
                                "unavailable": 0,
                                "untracked": 0,
                                "keys": 1,
                                "measurement_rate": 1.0,
                                "unavailable_reasons": {},
                            },
                            "python.tests": {
                                "total": 3,
                                "measured": 0,
                                "unavailable": 3,
                                "untracked": 0,
                                "keys": 0,
                                "measurement_rate": 0.0,
                                "unavailable_reasons": {
                                    "python.tests review has no exact test-node failure identity; node paths alone are not comparable": 3,
                                },
                            },
                        },
                        "unavailable_reasons": {
                            "python.tests review has no exact test-node failure identity; node paths alone are not comparable": 3,
                        },
                    },
                    "review_verification": {
                        "measured": 3,
                        "unavailable": 0,
                        "untracked": 0,
                        "keys": 2,
                        "unavailable_reasons": {},
                    },
                },
            }
        )
        self.assertIn("| `python.compile` | 3 | 3 | 0 | 0 | 1 | 1.0 |", report)
        self.assertIn("test-node failure identity", report)
        self.assertIn("does not estimate precision, recall, utility, or overlap", report)
        self.assertIn("Static `python.tests` overlap remains unavailable", report)
        self.assertIn("derive overlap from a test path", report)
        self.assertIn("Explicit review verification", report)
        self.assertIn("3 measured", report)


if __name__ == "__main__":
    unittest.main()
