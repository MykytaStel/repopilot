from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from scan_resource_matrix import (  # noqa: E402
    ScanResourcePolicyError,
    evaluate_matrix,
    load_scan_resource_policy,
    render_matrix,
)


def _policy() -> dict[str, object]:
    return {
        "schema_version": 1,
        "policy_id": "scan-rss-v1",
        "workload": "synthetic-medium-v1",
        "source": "posix-time-v1",
        "unit": "KiB",
        "required_scenarios": ["full_cold", "full_warm", "changed_cold", "changed_warm"],
        "ceiling_kb_by_scenario": {
            "full_cold": 500,
            "full_warm": 400,
            "changed_cold": 450,
            "changed_warm": 350,
        },
        "unavailable": "fail",
    }


def _samples(value: int = 100) -> dict[str, list[dict[str, object]]]:
    return {
        scenario: [
            {
                "resource_status": "available",
                "resource_source": "posix-time-v1",
                "child_max_rss_kb": value,
            }
            for _ in range(3)
        ]
        for scenario in ("full_cold", "full_warm", "changed_cold", "changed_warm")
    }


class ScanResourceMatrixTests(unittest.TestCase):
    def test_loads_and_validates_policy(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "policy.toml"
            path.write_text(
                """schema_version = 1
policy_id = "scan-rss-v1"
workload = "synthetic-medium-v1"
source = "posix-time-v1"
unit = "KiB"
required_scenarios = ["full_cold", "full_warm", "changed_cold", "changed_warm"]
ceiling_kb_by_scenario = { full_cold = 500, full_warm = 400, changed_cold = 450, changed_warm = 350 }
unavailable = "fail"
""",
                encoding="utf-8",
            )
            policy = load_scan_resource_policy(path)
        self.assertEqual(policy["policy_id"], "scan-rss-v1")
        self.assertEqual(policy["ceiling_kb_by_scenario"]["full_warm"], 400.0)

    def test_passes_and_renders_deterministically(self) -> None:
        result = evaluate_matrix(_samples(), _policy())
        self.assertEqual(result["status"], "pass")
        self.assertEqual(result["scenarios"]["full_cold"]["median_kb"], 100.0)
        self.assertEqual(render_matrix(result), render_matrix(result))
        json.dumps(result, sort_keys=True)

    def test_rejects_over_budget_and_unavailable_samples(self) -> None:
        samples = _samples(value=100)
        samples["full_cold"][0]["child_max_rss_kb"] = 600
        samples["changed_warm"][1] = {
            "resource_status": "unavailable",
            "resource_reason": "unsupported platform",
        }
        result = evaluate_matrix(samples, _policy())
        self.assertEqual(result["status"], "fail")
        kinds = {item["kind"] for item in result["violations"]}
        self.assertIn("max_over_ceiling", kinds)
        self.assertIn("unavailable", kinds)

    def test_rejects_missing_scenario_and_invalid_policy(self) -> None:
        samples = _samples()
        samples.pop("changed_cold")
        with self.assertRaisesRegex(ScanResourcePolicyError, "scenario set mismatch"):
            evaluate_matrix(samples, _policy())
        policy = _policy()
        policy["source"] = "other-v1"
        with self.assertRaisesRegex(ScanResourcePolicyError, "posix-time-v1"):
            evaluate_matrix(_samples(), policy)


if __name__ == "__main__":
    unittest.main()
