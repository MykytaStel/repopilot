from __future__ import annotations

import hashlib
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from differential_artifact import validate_data  # noqa: E402
from differential_runner import run_timed_command  # noqa: E402
from real_history_runner import BASELINE_COMMANDS  # noqa: E402


class DifferentialRunnerTests(unittest.TestCase):
    def test_timed_command_records_pass_and_hashes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = run_timed_command((sys.executable, "-c", "print('ok')"), Path(tmp), 5)
        self.assertEqual(result["status"], "passed")
        self.assertEqual(result["returncode"], 0)
        self.assertGreaterEqual(result["wall_ms"], 0)
        self.assertEqual(len(result["stdout_sha256"]), 64)

    def test_timed_command_records_failure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = run_timed_command((sys.executable, "-c", "raise SystemExit(3)"), Path(tmp), 5)
        self.assertEqual(result["status"], "failed")
        self.assertEqual(result["returncode"], 3)

    def test_artifact_validator_accepts_complete_pending_observations(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            holdout, differential, rules, zoo = self._write_manifests(root)
            cases = []
            for case_id, baseline_ids, base_sha, head_sha, merge_sha in (
                ("case-one", ("python.compile",), "a" * 40, "b" * 40, "c" * 40),
                ("case-two", ("python.tests",), "d" * 40, "e" * 40, "f" * 40),
            ):
                cases.append(
                    {
                        "id": case_id,
                        "repo": "owner/one" if case_id == "case-one" else "owner/two",
                        "base_sha": base_sha,
                        "head_sha": head_sha,
                        "merge_sha": merge_sha,
                        "baselines": {
                            baseline_id: [
                                {
                                    "command": list(BASELINE_COMMANDS[baseline_id]),
                                    "status": "passed",
                                    "wall_ms": 1.0,
                                }
                                for _ in range(3)
                            ]
                            for baseline_id in baseline_ids
                        },
                        "reviews": [
                            {"status": "collected", "wall_ms": 1.0, "stable_evidence_sha256": "1" * 64}
                            for _ in range(3)
                        ],
                        "determinism": {"stable_evidence_deterministic": True},
                    }
                )
            artifact = {
                "schema_version": 1,
                "corpus": "test",
                "protocol": "differential-utility-v1",
                "manifest_sha256": hashlib.sha256(holdout.read_bytes()).hexdigest(),
                "differential_manifest_sha256": hashlib.sha256(differential.read_bytes()).hexdigest(),
                "scanner": {"mode": "workspace", "version": "0.23", "report_schema_version": "1", "workspace_version": "0.23"},
                "repetitions": 3,
                "cases": cases,
                "label_state": "pending",
            }
            result = validate_data(artifact, holdout, differential, rules, zoo)
        self.assertEqual(result["status"], "valid")
        self.assertEqual(result["baseline_observations"], 6)
        self.assertEqual(result["review_observations"], 6)

    @staticmethod
    def _write_manifests(root: Path) -> tuple[Path, Path, Path, Path]:
        rules = root / "rules.md"
        zoo = root / "zoo.toml"
        holdout = root / "holdout.toml"
        differential = root / "differential.toml"
        rules.write_text("### `demo.rule` — Demo\n", encoding="utf-8")
        zoo.write_text("", encoding="utf-8")
        holdout.write_text(
            """schema_version = 1
corpus = "test"
protocol = "dual-independent-adjudication-v1"

[[case]]
id = "case-one"
repo = "owner/one"
url = "https://github.com/owner/one.git"
pull_request = 1
base_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
head_sha = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
merge_sha = "cccccccccccccccccccccccccccccccccccccccc"
language = "python"
source_kind = "merged-pull-request"
baseline_ids = ["python.compile"]
label_state = "pending"

[[case]]
id = "case-two"
repo = "owner/two"
url = "https://github.com/owner/two.git"
pull_request = 2
base_sha = "dddddddddddddddddddddddddddddddddddddddd"
head_sha = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
merge_sha = "ffffffffffffffffffffffffffffffffffffffff"
language = "python"
source_kind = "merged-pull-request"
baseline_ids = ["python.tests"]
label_state = "pending"
""",
            encoding="utf-8",
        )
        differential.write_text(
            """schema_version = 1
corpus = "test"
protocol = "differential-utility-v1"
repetitions = 3
measurements = ["novel-actionable-evidence", "duplicate-work", "time-to-first-useful-evidence", "decision-latency", "determinism", "resource-cost"]

[[case]]
id = "case-one"
baseline_ids = ["python.compile"]

[[case]]
id = "case-two"
baseline_ids = ["python.tests"]
""",
            encoding="utf-8",
        )
        return holdout, differential, rules, zoo


if __name__ == "__main__":
    unittest.main()
