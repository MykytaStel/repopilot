from __future__ import annotations

import json
import io
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest.mock import patch

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
PROJECT_ROOT = SCRIPTS_DIR.parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from differential_contract import DifferentialManifestError, validate_differential  # noqa: E402
from differential_budget import load_resource_policy  # noqa: E402
import differential  # noqa: E402


class DifferentialContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.holdout = self.root / "holdout.toml"
        self.rules = self.root / "rules.md"
        self.zoo = self.root / "zoo.toml"
        self.diff = self.root / "differential.toml"
        self.rules.write_text("### `demo.rule` — Demo\n", encoding="utf-8")
        self.zoo.write_text("", encoding="utf-8")
        self.holdout.write_text(
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
        self.diff.write_text(
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

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_accepts_frozen_metric_contract(self) -> None:
        result = validate_differential(self.diff, self.holdout, self.rules, self.zoo)
        self.assertEqual(result["status"], "valid")
        self.assertEqual(result["repetitions"], 3)
        self.assertEqual(result["measurements"][0], "decision-latency")

    def test_rejects_case_or_baseline_drift(self) -> None:
        text = self.diff.read_text(encoding="utf-8").replace('id = "case-two"', 'id = "case-other"')
        self.diff.write_text(text, encoding="utf-8")
        with self.assertRaisesRegex(DifferentialManifestError, "case set mismatch"):
            validate_differential(self.diff, self.holdout, self.rules, self.zoo)

    def test_rejects_single_repeat(self) -> None:
        text = self.diff.read_text(encoding="utf-8").replace("repetitions = 3", "repetitions = 1")
        self.diff.write_text(text, encoding="utf-8")
        with self.assertRaisesRegex(DifferentialManifestError, "at least 2"):
            validate_differential(self.diff, self.holdout, self.rules, self.zoo)

    def test_pilot_commands_require_their_artifacts(self) -> None:
        self.assertEqual(differential.main(["pilot-template", "--manifest", str(self.diff), "--holdout-manifest", str(self.holdout), "--rules-reference", str(self.rules), "--zoo-manifest", str(self.zoo)]), 2)
        self.assertEqual(differential.main(["pilot-validate", "--manifest", str(self.diff), "--holdout-manifest", str(self.holdout), "--rules-reference", str(self.rules), "--zoo-manifest", str(self.zoo)]), 2)
        self.assertEqual(differential.main(["pilot-score", "--manifest", str(self.diff), "--holdout-manifest", str(self.holdout), "--rules-reference", str(self.rules), "--zoo-manifest", str(self.zoo)]), 2)
        self.assertEqual(differential.main(["pilot-validate-metrics", "--manifest", str(self.diff), "--holdout-manifest", str(self.holdout), "--rules-reference", str(self.rules), "--zoo-manifest", str(self.zoo)]), 2)
        self.assertEqual(differential.main(["pilot-metrics-report", "--manifest", str(self.diff), "--holdout-manifest", str(self.holdout), "--rules-reference", str(self.rules), "--zoo-manifest", str(self.zoo)]), 2)
        self.assertEqual(differential.main(["coverage-audit", "--manifest", str(self.diff), "--holdout-manifest", str(self.holdout), "--rules-reference", str(self.rules), "--zoo-manifest", str(self.zoo)]), 2)

    def _budget_artifact(self, cold: int = 100, warm: int = 200) -> Path:
        self.diff.write_text(
            self.diff.read_text(encoding="utf-8")
            + """
[resource_policy]
schema_version = 1
policy_id = "differential-rss-v1"
workload = "test"
source = "posix-time-v1"
unit = "KiB"
statistic = "median"
required_phases = ["cold", "warm"]
ceiling_kb_by_phase = { cold = 500, warm = 400 }
unavailable = "fail"
""",
            encoding="utf-8",
        )
        artifact = {
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
                for case_id in ("case-one", "case-two")
            ],
        }
        path = self.root / "artifact.json"
        path.write_text(json.dumps(artifact), encoding="utf-8")
        return path

    def test_budget_check_passes_and_supports_json(self) -> None:
        artifact = self._budget_artifact()
        output = io.StringIO()
        with patch.object(differential, "validate_differential", return_value={"status": "valid"}), patch.object(
            differential, "validate_artifact", return_value={"status": "valid"}
        ), redirect_stdout(output):
            result = differential.main(
                [
                    "budget-check",
                    "--manifest",
                    str(self.diff),
                    "--holdout-manifest",
                    str(self.holdout),
                    "--rules-reference",
                    str(self.rules),
                    "--zoo-manifest",
                    str(self.zoo),
                    "--artifact",
                    str(artifact),
                    "--format",
                    "json",
                ]
            )
        self.assertEqual(result, 0)
        self.assertEqual(json.loads(output.getvalue())["status"], "pass")

    def test_budget_check_returns_failure_for_over_budget_artifact(self) -> None:
        artifact = self._budget_artifact(cold=600)
        output = io.StringIO()
        with patch.object(differential, "validate_differential", return_value={"status": "valid"}), patch.object(
            differential, "validate_artifact", return_value={"status": "valid"}
        ), redirect_stdout(output):
            result = differential.main(
                [
                    "budget-check",
                    "--manifest",
                    str(self.diff),
                    "--holdout-manifest",
                    str(self.holdout),
                    "--rules-reference",
                    str(self.rules),
                    "--zoo-manifest",
                    str(self.zoo),
                    "--artifact",
                    str(artifact),
                ]
            )
        self.assertEqual(result, 1)
        self.assertIn("Differential resource budget: fail", output.getvalue())

    def test_budget_check_is_unconfigured_without_policy(self) -> None:
        artifact = self.root / "artifact.json"
        artifact.write_text(json.dumps({"cases": []}), encoding="utf-8")
        output = io.StringIO()
        with patch.object(differential, "validate_differential", return_value={"status": "valid"}), patch.object(
            differential, "validate_artifact", return_value={"status": "valid"}
        ), redirect_stdout(output):
            result = differential.main(
                [
                    "budget-check",
                    "--manifest",
                    str(self.diff),
                    "--holdout-manifest",
                    str(self.holdout),
                    "--rules-reference",
                    str(self.rules),
                    "--zoo-manifest",
                    str(self.zoo),
                    "--artifact",
                    str(artifact),
                ]
            )
        self.assertEqual(result, 0)
        self.assertIn("unconfigured", output.getvalue())

    def test_budget_check_requires_artifact(self) -> None:
        error = io.StringIO()
        with redirect_stderr(error):
            result = differential.main(["budget-check", "--manifest", str(self.diff)])
        self.assertEqual(result, 2)
        self.assertIn("--artifact is required", error.getvalue())

    def test_budget_check_rejects_invalid_artifact(self) -> None:
        artifact = self._budget_artifact()
        error = io.StringIO()
        with patch.object(differential, "validate_differential", return_value={"status": "valid"}), patch.object(
            differential,
            "validate_artifact",
            side_effect=DifferentialManifestError("bad artifact"),
        ), redirect_stderr(error):
            result = differential.main(
                [
                    "budget-check",
                    "--manifest",
                    str(self.diff),
                    "--holdout-manifest",
                    str(self.holdout),
                    "--rules-reference",
                    str(self.rules),
                    "--zoo-manifest",
                    str(self.zoo),
                    "--artifact",
                    str(artifact),
                ]
            )
        self.assertEqual(result, 1)
        self.assertIn("bad artifact", error.getvalue())


class ProductionDifferentialManifestTests(unittest.TestCase):
    def test_expanded_differential_manifest_matches_holdout(self) -> None:
        result = validate_differential(
            PROJECT_ROOT / "tests/benchmarks/differential.toml",
            PROJECT_ROOT / "tests/benchmarks/manifest.toml",
            PROJECT_ROOT / "docs/rules-reference.md",
            PROJECT_ROOT / "tests/zoo/manifest.toml",
        )
        self.assertEqual(result["status"], "valid")
        self.assertEqual(result["corpus"], "v0.23-real-history-holdout-expanded")
        self.assertEqual(result["cases"], 6)
        self.assertEqual(result["repetitions"], 3)

    def test_production_manifest_declares_strict_resource_policy(self) -> None:
        policy = load_resource_policy(PROJECT_ROOT / "tests/benchmarks/differential.toml")
        self.assertIsNotNone(policy)
        self.assertEqual(policy["source"], "posix-time-v1")
        self.assertEqual(policy["required_phases"], ["cold", "warm"])
        self.assertEqual(policy["ceiling_kb_by_phase"], {"cold": 65536.0, "warm": 49152.0})


if __name__ == "__main__":
    unittest.main()
