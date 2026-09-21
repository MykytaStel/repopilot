from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from real_history_runner import BASELINE_COMMANDS, run_command, summarize_review  # noqa: E402


class RealHistoryRunnerTests(unittest.TestCase):
    def test_baseline_catalog_is_allowlisted(self) -> None:
        self.assertEqual(BASELINE_COMMANDS["python.compile"], ("python3", "-m", "compileall", "-q", "."))
        self.assertNotIn("shell", BASELINE_COMMANDS)

    def test_run_command_records_pass_and_output_hashes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = run_command(("python3", "-c", "print('ok')"), Path(tmp), 5)
        self.assertEqual(result["status"], "passed")
        self.assertEqual(result["returncode"], 0)
        self.assertEqual(len(result["stdout_sha256"]), 64)

    def test_summarize_review_separates_in_diff_rules(self) -> None:
        report = {
            "schema_version": "0.26",
            "repopilot_version": "0.22.0",
            "findings": [
                {"rule_id": "security.secret-candidate", "in_diff": True},
                {"rule_id": "architecture.large-file", "in_diff": False},
            ],
            "change_proof": {
                "contract_deltas": [
                    {"family": "security-boundary", "change": "boundary-changed"}
                ]
            },
        }
        summary = summarize_review(report, b"report")
        self.assertEqual(summary["in_diff_findings"], 1)
        self.assertEqual(summary["out_of_diff_findings"], 1)
        self.assertEqual(summary["in_diff_rule_ids"], ["security.secret-candidate"])
        self.assertEqual(summary["contract_delta_ids"], ["security-boundary/boundary-changed"])
        self.assertEqual(summary["contract_delta_count"], 1)
        self.assertEqual(len(summary["report_sha256"]), 64)
        self.assertEqual(len(summary["stable_evidence_sha256"]), 64)


if __name__ == "__main__":
    unittest.main()
