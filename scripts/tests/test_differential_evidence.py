from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from differential_evidence import normalize_baseline_evidence  # noqa: E402


class DifferentialEvidenceTests(unittest.TestCase):
    def test_compile_success_is_measured_with_empty_evidence(self) -> None:
        result = normalize_baseline_evidence("python.compile", b"", b"", 0, Path("/repo"))
        self.assertEqual(
            result,
            {
                "status": "measured",
                "keys": [],
                "source": "python.compile-v1",
                "comparison": {"status": "measured", "keys": [], "scheme": "review-exact-v1"},
            },
        )

    def test_compile_failure_normalizes_path_line_and_error_kind(self) -> None:
        stderr = b"""
*** Error compiling '/tmp/work/pkg/bad.py'...
  File "/tmp/work/pkg/bad.py", line 7
    broken(
          ^
SyntaxError: '(' was never closed
"""
        result = normalize_baseline_evidence("python.compile", b"", stderr, 1, Path("/tmp/work"))
        self.assertEqual(result["status"], "measured")
        self.assertEqual(result["keys"], ["python.compile:pkg/bad.py:7:SyntaxError"])
        self.assertEqual(
            result["comparison"],
            {
                "status": "measured",
                "keys": ["python.compile:pkg/bad.py:7:SyntaxError"],
                "scheme": "review-exact-v1",
            },
        )

    def test_compile_failure_without_supported_diagnostic_is_unavailable(self) -> None:
        result = normalize_baseline_evidence("python.compile", b"compiler crashed", b"", 1, Path("/repo"))
        self.assertEqual(result["status"], "unavailable")
        self.assertIn("supported diagnostic", result["reason"])

    def test_compile_diagnostic_outside_workspace_is_not_review_comparable(self) -> None:
        stderr = b"""
  File "/outside/bad.py", line 7
SyntaxError: invalid syntax
"""
        result = normalize_baseline_evidence("python.compile", b"", stderr, 1, Path("/repo"))
        self.assertEqual(result["status"], "measured")
        self.assertEqual(result["comparison"]["status"], "unavailable")

    def test_pytest_success_is_measured_with_empty_evidence(self) -> None:
        result = normalize_baseline_evidence("python.tests", b"3 passed in 0.02s\n", b"", 0, Path("/repo"))
        self.assertEqual(
            result,
            {
                "status": "measured",
                "keys": [],
                "source": "python.tests-v1",
                "comparison": {"status": "measured", "keys": [], "scheme": "review-exact-v1"},
            },
        )

    def test_pytest_failure_normalizes_node_ids_and_is_order_stable(self) -> None:
        output = b"""
FAILED tests/test_api.py::test_create - AssertionError
ERROR tests/test_db.py - ImportError: missing dependency
ERROR collecting tests/test_import.py
"""
        result = normalize_baseline_evidence("python.tests", output, b"", 1, Path("/repo"))
        self.assertEqual(result["status"], "measured")
        self.assertEqual(
            result["keys"],
            [
                "python.tests:tests/test_api.py::test_create:failed",
                "python.tests:tests/test_db.py:collection-error",
                "python.tests:tests/test_import.py:collection-error",
            ],
        )

    def test_pytest_unknown_failure_is_unavailable(self) -> None:
        result = normalize_baseline_evidence("python.tests", b"pytest crashed", b"", 2, Path("/repo"))
        self.assertEqual(result["status"], "unavailable")
        self.assertIn("supported diagnostic", result["reason"])

    def test_pytest_conftest_import_error_normalizes_collection_path(self) -> None:
        output = b"ImportError while loading conftest '/workspace/tests/conftest.py'.\n"
        result = normalize_baseline_evidence("python.tests", output, b"", 4, Path("/workspace"))
        self.assertEqual(result["status"], "measured")
        self.assertEqual(result["keys"], ["python.tests:tests/conftest.py:collection-error"])

    def test_unknown_baseline_is_unavailable(self) -> None:
        result = normalize_baseline_evidence("python.lint", b"", b"", 0, Path("/repo"))
        self.assertEqual(result["status"], "unavailable")
        self.assertIn("no adapter", result["reason"])


if __name__ == "__main__":
    unittest.main()
