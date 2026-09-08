from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from differential_identity import review_comparable_diagnostic_keys  # noqa: E402


class DifferentialIdentityTests(unittest.TestCase):
    def test_python_syntax_diagnostic_maps_only_to_a_changed_line(self) -> None:
        report = {
            "root_path": "/worktree",
            "changed_files": [
                {"path": "pkg/bad.py", "ranges": [{"start": 7, "end": 7}]},
            ],
            "diagnostics": [
                {
                    "code": "python.syntax-error",
                    "path": "/worktree/pkg/bad.py",
                    "line": 7,
                },
            ],
        }

        self.assertEqual(
            review_comparable_diagnostic_keys(report),
            ["python.compile:pkg/bad.py:7:SyntaxError"],
        )

    def test_python_syntax_diagnostic_outside_diff_is_not_comparable(self) -> None:
        report = {
            "root_path": "/worktree",
            "changed_files": [
                {"path": "pkg/bad.py", "ranges": [{"start": 8, "end": 8}]},
            ],
            "diagnostics": [
                {
                    "code": "python.syntax-error",
                    "path": "/worktree/pkg/bad.py",
                    "line": 7,
                },
            ],
        }

        self.assertEqual(review_comparable_diagnostic_keys(report), [])

    def test_unknown_diagnostic_code_and_path_escape_are_ignored(self) -> None:
        report = {
            "root_path": "/worktree",
            "changed_files": [
                {"path": "pkg/bad.py", "ranges": [{"start": 7, "end": 7}]},
            ],
            "diagnostics": [
                {
                    "code": "syntax.parse-error",
                    "path": "/worktree/pkg/bad.py",
                    "line": 7,
                },
                {
                    "code": "python.syntax-error",
                    "path": "/outside/bad.py",
                    "line": 7,
                },
            ],
        }

        self.assertEqual(review_comparable_diagnostic_keys(report), [])


if __name__ == "__main__":
    unittest.main()
