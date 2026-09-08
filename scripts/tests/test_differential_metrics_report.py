from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from differential_metrics_report import render_differential_metrics_report  # noqa: E402


class DifferentialMetricsReportTests(unittest.TestCase):
    def test_render_exposes_cold_and_warm_rss_separately(self) -> None:
        report = render_differential_metrics_report(
            {
                "corpus": "holdout",
                "protocol": "single-expert-pilot-v1",
                "reviewer": "expert",
                "scope": "exploratory",
                "limitation": "limited corpus",
                "counts": {"tp": 0, "fn": 0, "tn": 1, "fp": 0, "excluded": 0},
                "measurements": {
                    "case_count": 1,
                    "deterministic_cases": 1,
                    "novel_evidence_cases": 0,
                    "median_baseline_wall_ms": 10.0,
                    "median_review_wall_ms": 20.0,
                    "median_review_child_max_rss_kb": 120.0,
                    "median_review_child_max_rss_kb_by_phase": {"cold": 150.0, "warm": 100.0},
                    "time_to_first_useful_evidence": {"status": "unavailable", "reason": "not recorded"},
                    "decision_latency": {"status": "unavailable", "reason": "not recorded"},
                    "duplicate_work": {"status": "unavailable", "reason": "unlabeled"},
                },
                "cases": [
                    {
                        "id": "case-one",
                        "outcome": "tn",
                        "expected_rule_ids": [],
                        "observed_novel_rule_ids": [],
                        "novel_evidence_count": 0,
                        "measurements": {
                            "median_review_wall_ms": 20.0,
                            "median_review_child_max_rss_kb_by_phase": {"cold": 150.0, "warm": 100.0},
                        },
                    }
                ],
            }
        )
        self.assertIn("Median review peak RSS cold (KiB)", report)
        self.assertIn("Median review peak RSS warm (KiB)", report)
        self.assertIn("RSS cold KiB", report)
        self.assertIn("| 0 | 20.000 | 150.000 | 100.000 |", report)


if __name__ == "__main__":
    unittest.main()
