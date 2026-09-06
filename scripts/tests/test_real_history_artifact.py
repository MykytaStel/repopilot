from __future__ import annotations

import hashlib
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from real_history_artifact import validate_collection_data  # noqa: E402
from real_history_contract import validate_manifest  # noqa: E402
from real_history_runner import BASELINE_COMMANDS  # noqa: E402


class CollectionArtifactTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.manifest = self.root / "manifest.toml"
        self.rules = self.root / "rules.md"
        self.zoo = self.root / "zoo.toml"
        self.rules.write_text("### `demo.rule` — Demo\n", encoding="utf-8")
        self.zoo.write_text("", encoding="utf-8")
        self.manifest.write_text(
            """schema_version = 1
corpus = "test"
protocol = "dual-v1"

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
baseline_ids = ["python.compile"]
label_state = "pending"
""",
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def artifact(self) -> dict[str, object]:
        corpus, protocol, cases = validate_manifest(self.manifest, self.rules, self.zoo)
        observations = []
        for case in cases:
            command = list(BASELINE_COMMANDS["python.compile"])
            observations.append(
                {
                    "id": case.case_id,
                    "repo": case.repo,
                    "pull_request": case.pull_request,
                    "base_sha": case.base_sha,
                    "head_sha": case.head_sha,
                    "merge_sha": case.merge_sha,
                    "label_state": case.label_state,
                    "baselines": {"python.compile": {"status": "passed", "command": command}},
                    "review": {"status": "collected"},
                }
            )
        return {
            "schema_version": 1,
            "corpus": corpus,
            "protocol": protocol,
            "manifest_sha256": hashlib.sha256(self.manifest.read_bytes()).hexdigest(),
            "scanner": {
                "mode": "explicit",
                "version": "0.22.0",
                "report_schema_version": "0.26",
                "workspace_version": "0.22.0",
            },
            "cases": observations,
            "label_state": "pending",
        }

    def test_accepts_matching_collection(self) -> None:
        result = validate_collection_data(self.artifact(), self.manifest, self.rules, self.zoo)
        self.assertEqual(result["status"], "valid")
        self.assertEqual(result["cases"], 2)

    def test_rejects_manifest_hash_drift(self) -> None:
        artifact = self.artifact()
        artifact["manifest_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "manifest_sha256"):
            validate_collection_data(artifact, self.manifest, self.rules, self.zoo)

    def test_rejects_baseline_command_drift(self) -> None:
        artifact = self.artifact()
        artifact["cases"][0]["baselines"]["python.compile"]["command"] = ["sh", "-c", "unsafe"]
        with self.assertRaisesRegex(ValueError, "baseline command drift"):
            validate_collection_data(artifact, self.manifest, self.rules, self.zoo)


if __name__ == "__main__":
    unittest.main()
