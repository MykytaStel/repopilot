from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from differential_budget import (  # noqa: E402
    DifferentialBudgetError,
    evaluate_resource_budget,
    load_resource_policy,
    render_resource_budget,
)


def _policy() -> dict[str, object]:
    return {
        "schema_version": 1,
        "policy_id": "differential-rss-v1",
        "workload": "holdout",
        "source": "posix-time-v1",
        "unit": "KiB",
        "statistic": "median",
        "required_phases": ["cold", "warm"],
        "ceiling_kb_by_phase": {"cold": 500, "warm": 400},
        "unavailable": "fail",
    }


def _manifest(*case_ids: str) -> dict[str, object]:
    return {"case": [{"id": case_id} for case_id in case_ids]}


def _artifact(*case_ids: str, cold: int = 100, warm: int = 200) -> dict[str, object]:
    return {
        "schema_version": 2,
        "cases": [
            {
                "id": case_id,
                "reviews": [
                    {
                        "resource_status": "available",
                        "resource_source": "posix-time-v1",
                        "resource_phase": "cold",
                        "child_max_rss_kb": cold,
                    },
                    {
                        "resource_status": "available",
                        "resource_source": "posix-time-v1",
                        "resource_phase": "warm",
                        "child_max_rss_kb": warm,
                    },
                ],
            }
            for case_id in case_ids
        ],
    }


class DifferentialBudgetTests(unittest.TestCase):
    def test_passes_and_reports_phase_statistics(self) -> None:
        result = evaluate_resource_budget(_artifact("one", "two"), _manifest("one", "two"), _policy())
        self.assertEqual(result["status"], "pass")
        self.assertEqual(result["phase_summary"]["cold"]["median_kb"], 100.0)
        self.assertEqual(result["phase_summary"]["warm"]["max_kb"], 200.0)
        self.assertEqual(result["violations"], [])

    def test_rejects_cold_median_over_ceiling(self) -> None:
        result = evaluate_resource_budget(_artifact("one", cold=600), _manifest("one"), _policy())
        self.assertEqual(result["status"], "fail")
        self.assertTrue(any(item["kind"] == "median_over_ceiling" for item in result["violations"]))

    def test_rejects_warm_maximum_over_ceiling(self) -> None:
        artifact = _artifact("one", warm=200)
        artifact["cases"][0]["reviews"].append(
            {
                "resource_status": "available",
                "resource_source": "posix-time-v1",
                "resource_phase": "warm",
                "child_max_rss_kb": 450,
            }
        )
        result = evaluate_resource_budget(artifact, _manifest("one"), _policy())
        self.assertEqual(result["status"], "fail")
        self.assertTrue(any(item["kind"] == "max_over_ceiling" for item in result["violations"]))

    def test_rejects_missing_phase_and_unavailable_sample(self) -> None:
        artifact = _artifact("one")
        artifact["cases"][0]["reviews"][1]["resource_status"] = "unavailable"
        artifact["cases"][0]["reviews"][1]["resource_reason"] = "unsupported platform"
        artifact["cases"][0]["reviews"][1].pop("resource_phase")
        result = evaluate_resource_budget(artifact, _manifest("one"), _policy())
        kinds = {item["kind"] for item in result["violations"]}
        self.assertIn("missing_phase", kinds)
        self.assertIn("unavailable", kinds)

    def test_rejects_unsupported_source(self) -> None:
        artifact = _artifact("one")
        artifact["cases"][0]["reviews"][0]["resource_source"] = "other-v1"
        result = evaluate_resource_budget(artifact, _manifest("one"), _policy())
        self.assertTrue(any(item["kind"] == "source_mismatch" for item in result["violations"]))

    def test_rejects_manifest_case_drift(self) -> None:
        with self.assertRaisesRegex(DifferentialBudgetError, "case set mismatch"):
            evaluate_resource_budget(_artifact("one"), _manifest("two"), _policy())

    def test_render_is_deterministic_and_json_safe(self) -> None:
        result = evaluate_resource_budget(_artifact("one"), _manifest("one"), _policy())
        rendered = render_resource_budget(result)
        self.assertEqual(rendered, render_resource_budget(result))
        self.assertIn("Differential resource budget: pass", rendered)
        json.dumps(result, sort_keys=True)

    def test_load_policy_is_optional_and_validated(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "manifest.toml"
            path.write_text("schema_version = 1\n", encoding="utf-8")
            self.assertIsNone(load_resource_policy(path))
            path.write_text(
                """[resource_policy]
schema_version = 1
policy_id = "differential-rss-v1"
workload = "holdout"
source = "posix-time-v1"
unit = "KiB"
statistic = "median"
required_phases = ["cold", "warm"]
ceiling_kb_by_phase = { cold = 500, warm = 400 }
unavailable = "fail"
""",
                encoding="utf-8",
            )
            self.assertEqual(load_resource_policy(path)["policy_id"], "differential-rss-v1")

    def test_rejects_invalid_policy(self) -> None:
        policy = _policy()
        policy["required_phases"] = ["cold"]
        with self.assertRaisesRegex(DifferentialBudgetError, "required phases"):
            evaluate_resource_budget(_artifact("one"), _manifest("one"), policy)


if __name__ == "__main__":
    unittest.main()
