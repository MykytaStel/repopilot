"""Release-gate tests with validated-artifact shaped synthetic observations."""

import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from changeproof_benchmark_gate import evaluate_gate
from changeproof_benchmark_contract import BenchmarkManifestError


def observed(binary):
    evaluation = {"decision": {"status": "pass"}}
    runs = [
        {
            "phase": phase,
            "evaluation": evaluation,
            "signal_kinds": ["definitely/test.example"],
            "wall_ms": 100,
            "resource": {"status": "available", "peak_rss_kb": 1000},
        }
        for phase in ("cold", "warm", "warm")
    ]
    return {
        "artifact": {
            "protocol": "changeproof-benchmark-v1",
            "corpus": "test",
            "manifest_sha256": "manifest",
            "scanner": {"binary_sha256": binary},
            "cases": [{
                "id": "example", "fixture": "example", "variant": "unsafe",
                "fixture_source_sha256": "source", "oracle": {},
                "signal_oracle": {"sha256": "oracle"},
                "deterministic": True, "proof_semantic_hashes": ["proof"], "runs": runs,
            }],
        },
        "summary": {
            "case_count": 1, "deterministic_cases": 1, "false_certainty_cases": 0,
            "dimensions": {"decision": {"failed": 0}},
            "signals": {
                "measured_cases": 1, "unavailable_cases": 0, "nondeterministic_cases": 0,
                "positive_expected": 1, "positive_detected": 1, "safe_cases": 1,
                "misses": [], "safe_alerts": [],
            },
        },
    }


class ReleaseGateTests(unittest.TestCase):
    def setUp(self):
        self.baseline = observed("a")
        self.candidate = observed("b")

    def codes(self):
        result = evaluate_gate(self.baseline, self.candidate)
        self.assertEqual(result["status"], "fail")
        return {item["code"] for item in result["failures"]}

    def test_matching_observations_pass(self):
        self.assertEqual(evaluate_gate(self.baseline, self.candidate)["status"], "pass")

    def test_signal_oracle_change_rejected(self):
        self.candidate["artifact"]["cases"][0]["signal_oracle"]["sha256"] = "other"
        with self.assertRaises(BenchmarkManifestError):
            evaluate_gate(self.baseline, self.candidate)

    def test_same_count_replaced_miss_is_a_regression(self):
        self.baseline["summary"]["signals"]["misses"] = [{"id": "a", "signals": ["x"]}]
        self.candidate["summary"]["signals"]["misses"] = [{"id": "b", "signals": ["x"]}]
        self.assertIn("new-misses", self.codes())

    def test_missing_measurements_and_false_certainty_fail(self):
        summary = self.candidate["summary"]
        summary["false_certainty_cases"] = 1
        summary["signals"]["measured_cases"] = 0
        self.assertTrue({"false-certainty", "candidate-signal-evidence-incomplete"} <= self.codes())

    def test_budget_failure_and_unavailable_rss(self):
        runs = self.candidate["artifact"]["cases"][0]["runs"]
        runs[0]["wall_ms"] = 111
        runs[1]["resource"] = {"status": "unavailable"}
        self.assertTrue({"cold-wall-budget", "warm-rss-budget"} <= self.codes())

    def test_controlled_recall_floor(self):
        self.candidate["summary"]["signals"]["positive_expected"] = 10
        self.candidate["summary"]["signals"]["positive_detected"] = 8
        self.assertIn("controlled-positive-recall-below-85pct", self.codes())

    def test_new_safe_alert_fails(self):
        self.candidate["summary"]["signals"]["safe_alerts"] = [{"id": "safe", "signals": ["x"]}]
        self.assertIn("new-safe_alerts", self.codes())

    def test_identical_binary_rejected(self):
        self.candidate = copy.deepcopy(self.baseline)
        with self.assertRaises(BenchmarkManifestError):
            evaluate_gate(self.baseline, self.candidate)


if __name__ == "__main__":
    unittest.main()
