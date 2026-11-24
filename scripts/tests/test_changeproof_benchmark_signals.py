from __future__ import annotations

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from changeproof_benchmark_contract import BenchmarkManifestError
from changeproof_benchmark_signals import normalize_signal_kinds, summarize_signals, validate_signal_kinds


class ChangeProofSignalTests(unittest.TestCase):
    def test_projection_is_ordered_deduplicated_and_source_free(self):
        report = {"tiered_signals": {
            "definitely": [{"kind": "taint.sql", "detail": "private source"},
                           {"kind": "taint.sql"}],
            "maybe": [{"kind": "behavioral.network-call-added"}],
            "noise": [],
        }}
        self.assertEqual(normalize_signal_kinds(report),
                         ["definitely/taint.sql", "maybe/behavioral.network-call-added"])

    def test_missing_tier_and_malformed_identity_are_rejected(self):
        with self.assertRaisesRegex(BenchmarkManifestError, "unavailable"):
            normalize_signal_kinds({"tiered_signals": {"definitely": [], "maybe": []}})
        for value in (["definitely/taint.sql", "definitely/taint.sql"], ["raw source"], ["other/taint.sql"]):
            with self.assertRaises(BenchmarkManifestError):
                validate_signal_kinds(value)

    def test_metrics_separate_missing_positive_safe_alerts_and_drift(self):
        def case(case_id, variant, expected, runs, deterministic=True):
            return {
                "case": SimpleNamespace(case_id=case_id, variant=variant),
                "signal_oracle": {"expected": expected},
                "runs": [{"signal_kinds": value} for value in runs],
                "deterministic": deterministic,
            }
        sql = "definitely/taint.sql"
        extra = "maybe/behavioral.network-call-added"
        results = summarize_signals([
            case("detected", "unsafe", [sql], [[extra, sql]] * 3),
            case("missed", "unsafe", [sql], [[]] * 3),
            case("safe", "safe", [], [[sql]] * 3),
            case("clean", "safe", [], [[]] * 3),
            case("drift", "unsafe", [sql], [[sql], [], [sql]]),
            {"case": SimpleNamespace(case_id="legacy", variant="safe"), "runs": [], "deterministic": True},
        ])
        self.assertEqual(results["measured_cases"], 4)
        self.assertEqual(results["positive_expected"], 2)
        self.assertEqual(results["positive_detected"], 1)
        self.assertEqual(results["safe_cases_with_signals"], 1)
        self.assertEqual(results["safe_cases"], 2)
        self.assertEqual(results["nondeterministic_cases"], 1)
        self.assertEqual(results["unavailable_cases"], 1)
        self.assertNotIn(extra, results["by_signal"])


if __name__ == "__main__":
    unittest.main()
