from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from differential_contract import DifferentialManifestError, validate_differential  # noqa: E402
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


if __name__ == "__main__":
    unittest.main()
