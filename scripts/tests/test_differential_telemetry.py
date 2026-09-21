from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from differential_telemetry import (  # noqa: E402
    build_command_telemetry,
    build_review_telemetry,
    validate_telemetry,
)


class DifferentialTelemetryTests(unittest.TestCase):
    def test_review_telemetry_records_evidence_and_decision_events(self) -> None:
        report = {
            "scan_timings": {"file_analysis_us": 2500, "report_finalization_us": 500},
            "review_timings": {
                "diff_loading_us": 1000,
                "review_signals_us": 2000,
                "gating_us": 1000,
                "verification_us": 500,
            },
        }
        telemetry = build_review_telemetry(report, 12.0, useful_evidence=True)
        self.assertEqual(
            [event["name"] for event in telemetry["events"]],
            ["process_started", "evidence_ready", "decision_ready", "process_finished"],
        )
        self.assertEqual(telemetry["events"][1]["elapsed_ms"], 6.0)
        self.assertEqual(telemetry["events"][2]["elapsed_ms"], 7.5)
        validate_telemetry(telemetry, 12.0, "review")

    def test_command_telemetry_has_bounded_start_and_finish_events(self) -> None:
        telemetry = build_command_telemetry(4.25)
        self.assertEqual(telemetry["events"], [
            {"name": "process_started", "elapsed_ms": 0.0},
            {"name": "process_finished", "elapsed_ms": 4.25},
        ])
        validate_telemetry(telemetry, 4.25, "baseline")

    def test_review_without_novel_evidence_does_not_invent_evidence_event(self) -> None:
        report = {
            "scan_timings": {"file_analysis_us": 1000},
            "review_timings": {"diff_loading_us": 1000, "review_signals_us": 1000},
        }
        telemetry = build_review_telemetry(report, 8.0, useful_evidence=False)
        self.assertNotIn("evidence_ready", [event["name"] for event in telemetry["events"]])
        self.assertIn("decision_ready", [event["name"] for event in telemetry["events"]])
        validate_telemetry(telemetry, 8.0, "review")

    def test_validator_rejects_non_monotonic_events(self) -> None:
        telemetry = build_command_telemetry(4.25)
        telemetry["events"].insert(1, {"name": "decision_ready", "elapsed_ms": 5.0})
        with self.assertRaisesRegex(ValueError, "monotonic"):
            validate_telemetry(telemetry, 4.25, "review")


if __name__ == "__main__":
    unittest.main()
