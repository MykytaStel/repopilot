from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-review-quality.py"
FIXTURES = Path(__file__).resolve().parent / "fixtures"
SPEC = importlib.util.spec_from_file_location("check_review_quality", SCRIPT)
assert SPEC and SPEC.loader
check_review_quality = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check_review_quality)


def report_with(signal: dict) -> dict:
    return {
        "tiered_signals": {
            "definitely": [signal],
            "maybe": [],
            "noise": [],
        }
    }


class ReviewQualityTests(unittest.TestCase):
    def test_regression_fixtures_define_clean_and_rejected_reports(self) -> None:
        clean = check_review_quality.load_report(FIXTURES / "review-quality-clean.json")
        rejected = check_review_quality.load_report(
            FIXTURES / "review-quality-rejected.json"
        )

        self.assertEqual(check_review_quality.quality_failures(clean), [])
        self.assertEqual(len(check_review_quality.quality_failures(rejected)), 3)

    def test_rejects_runtime_side_effects_from_test_paths(self) -> None:
        for path in (
            "tests/fixtures/client.test.ts",
            "src/behavioral_tests.rs",
            "fixtures/runtime/client.rs",
        ):
            failures = check_review_quality.quality_failures(
                report_with(
                    {
                        "kind": "behavioral.network-call-added",
                        "path": path,
                    }
                )
            )

            self.assertEqual(len(failures), 1)
            self.assertIn("must not surface", failures[0])

    def test_rejects_coarse_removed_behavior_from_prose(self) -> None:
        failures = check_review_quality.quality_failures(
            report_with(
                {
                    "kind": "behavioral.auth-check-removed",
                    "path": "docs/security.md",
                    "provenance": {"signal_source": "text-heuristic"},
                }
            )
        )

        self.assertEqual(len(failures), 1)
        self.assertIn("coarse fallback", failures[0])

    def test_rejects_standard_library_dependency_imports(self) -> None:
        failures = check_review_quality.quality_failures(
            report_with(
                {
                    "kind": "behavioral.dependency-import-added",
                    "path": "scripts/check.py",
                    "detail": "Imported module: from pathlib import Path",
                }
            )
        )

        self.assertEqual(len(failures), 1)
        self.assertIn("standard-library import", failures[0])

    def test_accepts_a_runtime_signal_from_product_code(self) -> None:
        failures = check_review_quality.quality_failures(
            report_with(
                {
                    "kind": "behavioral.network-call-added",
                    "path": "src/client.ts",
                }
            )
        )

        self.assertEqual(failures, [])


if __name__ == "__main__":
    unittest.main()
