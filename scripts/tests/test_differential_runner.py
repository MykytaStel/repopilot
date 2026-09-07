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
from differential_telemetry import build_command_telemetry  # noqa: E402
from real_history_runner import BASELINE_COMMANDS  # noqa: E402


class DifferentialRunnerTests(unittest.TestCase):
    def test_timed_command_records_pass_and_hashes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = run_timed_command((sys.executable, "-c", "print('ok')"), Path(tmp), 5)
        self.assertEqual(result["status"], "passed")
        self.assertEqual(result["returncode"], 0)
        self.assertGreaterEqual(result["wall_ms"], 0)
        self.assertEqual(len(result["stdout_sha256"]), 64)
        self.assertEqual(result["telemetry"]["events"][0]["name"], "process_started")
        self.assertEqual(result["telemetry"]["events"][-1]["name"], "process_finished")

    def test_timed_command_records_failure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = run_timed_command((sys.executable, "-c", "raise SystemExit(3)"), Path(tmp), 5)
        self.assertEqual(result["status"], "failed")
        self.assertEqual(result["returncode"], 3)

    def test_timed_baseline_command_records_normalized_evidence(self) -> None:
        diagnostic = (
            "*** Error compiling '/workspace/pkg/bad.py'...\n"
            "  File '/workspace/pkg/bad.py', line 4\n"
            "SyntaxError: invalid syntax\n"
        )
        with tempfile.TemporaryDirectory() as tmp:
            result = run_timed_command(
                (
                    sys.executable,
                    "-c",
                    "import sys; print(sys.argv[1], file=sys.stderr); raise SystemExit(1)",
                    diagnostic,
                ),
                Path(tmp),
                5,
                "python.compile",
            )
        self.assertEqual(result["evidence"]["status"], "measured")
        self.assertEqual(result["evidence"]["keys"], ["python.compile:bad.py:4:SyntaxError"])

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
                        "base_scan": {"status": "collected", "wall_ms": 1.0, "evidence_keys": []},
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
                            {
                                "status": "collected",
                                "wall_ms": 1.0,
                                "stable_evidence_sha256": "1" * 64,
                                "in_diff_evidence_keys": [],
                                "novel_in_diff_evidence_keys": [],
                            }
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

    def test_artifact_validator_requires_base_scan_for_novelty(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            holdout, differential, rules, zoo = self._write_manifests(root)
            artifact = self._valid_artifact(holdout, differential)
            artifact["cases"][0].pop("base_scan")
            with self.assertRaisesRegex(ValueError, "base scan"):
                validate_data(artifact, holdout, differential, rules, zoo)

    def test_artifact_schema_two_requires_telemetry_for_every_run(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            holdout, differential, rules, zoo = self._write_manifests(root)
            artifact = self._valid_artifact(holdout, differential)
            artifact["schema_version"] = 2
            with self.assertRaisesRegex(ValueError, "telemetry"):
                validate_data(artifact, holdout, differential, rules, zoo)

    def test_artifact_schema_two_accepts_valid_telemetry_and_baseline_evidence_status(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            holdout, differential, rules, zoo = self._write_manifests(root)
            artifact = self._valid_artifact(holdout, differential)
            artifact["schema_version"] = 2
            for case in artifact["cases"]:
                case["base_scan"]["telemetry"] = build_command_telemetry(1.0)
                for runs in case["baselines"].values():
                    for run in runs:
                        run["telemetry"] = build_command_telemetry(1.0)
                        run["evidence"] = {
                            "status": "unavailable",
                            "reason": "fixture has no baseline adapter",
                        }
                for review in case["reviews"]:
                    review["telemetry"] = build_command_telemetry(1.0)
            result = validate_data(artifact, holdout, differential, rules, zoo)
        self.assertEqual(result["status"], "valid")

    def test_artifact_schema_two_requires_provenance_for_measured_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            holdout, differential, rules, zoo = self._write_manifests(root)
            artifact = self._valid_artifact(holdout, differential)
            artifact["schema_version"] = 2
            for case in artifact["cases"]:
                case["base_scan"]["telemetry"] = build_command_telemetry(1.0)
                for runs in case["baselines"].values():
                    for run in runs:
                        run["telemetry"] = build_command_telemetry(1.0)
                        run["evidence"] = {"status": "measured", "keys": []}
                for review in case["reviews"]:
                    review["telemetry"] = build_command_telemetry(1.0)
            with self.assertRaisesRegex(ValueError, "evidence source"):
                validate_data(artifact, holdout, differential, rules, zoo)
            for case in artifact["cases"]:
                for runs in case["baselines"].values():
                    for run in runs:
                        run["evidence"]["source"] = "unrelated-adapter-v1"
            with self.assertRaisesRegex(ValueError, "source drift"):
                validate_data(artifact, holdout, differential, rules, zoo)

    @staticmethod
    def _valid_artifact(holdout: Path, differential: Path) -> dict[str, object]:
        cases = []
        for case_id, baseline_ids, repo, base_sha, head_sha, merge_sha in (
            ("case-one", ("python.compile",), "owner/one", "a" * 40, "b" * 40, "c" * 40),
            ("case-two", ("python.tests",), "owner/two", "d" * 40, "e" * 40, "f" * 40),
        ):
            cases.append(
                {
                    "id": case_id,
                    "repo": repo,
                    "base_sha": base_sha,
                    "head_sha": head_sha,
                    "merge_sha": merge_sha,
                    "base_scan": {"status": "collected", "wall_ms": 1.0, "evidence_keys": []},
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
                        {
                            "status": "collected",
                            "wall_ms": 1.0,
                            "stable_evidence_sha256": "1" * 64,
                            "in_diff_evidence_keys": [],
                            "novel_in_diff_evidence_keys": [],
                        }
                        for _ in range(3)
                    ],
                    "determinism": {"stable_evidence_deterministic": True},
                }
            )
        return {
            "schema_version": 1,
            "corpus": "test",
            "protocol": "differential-utility-v1",
            "manifest_sha256": hashlib.sha256(holdout.read_bytes()).hexdigest(),
            "differential_manifest_sha256": hashlib.sha256(differential.read_bytes()).hexdigest(),
            "scanner": {
                "mode": "workspace",
                "version": "0.23",
                "report_schema_version": "1",
                "workspace_version": "0.23",
            },
            "repetitions": 3,
            "cases": cases,
            "label_state": "pending",
        }

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
