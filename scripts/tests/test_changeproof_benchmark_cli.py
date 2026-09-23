from __future__ import annotations

import contextlib
import io
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import changeproof_benchmark  # noqa: E402


class ChangeProofBenchmarkCliTests(unittest.TestCase):
    def test_collect_requires_output_and_positive_timeout(self) -> None:
        self.assertEqual(changeproof_benchmark.main(["collect"]), 2)
        self.assertEqual(changeproof_benchmark.main(["collect", "--output", "result.json", "--timeout", "0"]), 2)

    def test_report_refuses_unvalidated_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            artifact = root / "tampered.json"
            output = root / "report.md"
            artifact.write_text("not json", encoding="utf-8")

            with contextlib.redirect_stderr(io.StringIO()):
                result = changeproof_benchmark.main(["report", "--artifact", str(artifact), "--output", str(output)])

        self.assertEqual(result, 1)

    def test_check_reports_the_committed_protocol(self) -> None:
        output = io.StringIO()

        with contextlib.redirect_stdout(output):
            result = changeproof_benchmark.main(["check"])

        self.assertEqual(result, 0)
        self.assertIn("changeproof-benchmark-v1", output.getvalue())
        self.assertIn("8 cases", output.getvalue())

    def test_collect_validates_before_writing(self) -> None:
        with tempfile.TemporaryDirectory() as temporary, patch(
            "changeproof_benchmark.collect_benchmark", return_value={"invalid": True}
        ):
            output = Path(temporary) / "result.json"
            with contextlib.redirect_stderr(io.StringIO()):
                result = changeproof_benchmark.main(["collect", "--scanner", "scanner", "--output", str(output)])
            self.assertFalse(output.exists())

        self.assertEqual(result, 1)


if __name__ == "__main__":
    unittest.main()
