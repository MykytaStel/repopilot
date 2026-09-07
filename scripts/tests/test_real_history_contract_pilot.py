from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from real_history_contract_pilot import render_contract_pilot_template, validate_contract_pilot  # noqa: E402
from real_history_contract_pilot_metrics import build_contract_pilot_metrics  # noqa: E402
from real_history_contracts import contract_evidence_hash  # noqa: E402
from real_history_contract import validate_manifest  # noqa: E402
from real_history_runner import BASELINE_COMMANDS  # noqa: E402


class ContractPilotTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.manifest = self.root / "manifest.toml"
        self.rules = self.root / "rules.md"
        self.zoo = self.root / "zoo.toml"
        self.collection = self.root / "collection.json"
        self.rules.write_text("### `demo.rule` — Demo\n", encoding="utf-8")
        self.zoo.write_text("", encoding="utf-8")
        self.manifest.write_text(
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
baseline_ids = ["python.compile"]
label_state = "pending"
""",
            encoding="utf-8",
        )
        _, protocol, cases = validate_manifest(self.manifest, self.rules, self.zoo)
        observations = []
        contract_id = "security-boundary/boundary-changed"
        for case in cases:
            ids = [contract_id] if case.case_id == "case-one" else []
            observations.append(
                {
                    "id": case.case_id,
                    "repo": case.repo,
                    "pull_request": case.pull_request,
                    "base_sha": case.base_sha,
                    "head_sha": case.head_sha,
                    "merge_sha": case.merge_sha,
                    "label_state": "pending",
                    "baselines": {
                        "python.compile": {
                            "status": "passed",
                            "command": list(BASELINE_COMMANDS["python.compile"]),
                        }
                    },
                    "review": {
                        "status": "collected",
                        "contract_delta_ids": ids,
                        "contract_delta_count": len(ids),
                        "contract_evidence_sha256": contract_evidence_hash(ids),
                    },
                }
            )
        self.collection.write_text(
            json.dumps(
                {
                    "schema_version": 2,
                    "corpus": "test",
                    "protocol": protocol,
                    "manifest_sha256": hashlib.sha256(self.manifest.read_bytes()).hexdigest(),
                    "scanner": {
                        "mode": "explicit",
                        "version": "0.23.0",
                        "report_schema_version": "0.26",
                        "workspace_version": "0.23.0",
                    },
                    "cases": observations,
                    "label_state": "pending",
                }
            ),
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_template_is_blinded_and_validates_after_labels(self) -> None:
        pilot = self.root / "pilot.toml"
        pilot.write_text(
            render_contract_pilot_template(
                self.collection, self.manifest, self.rules, self.zoo, "expert"
            ),
            encoding="utf-8",
        )
        rendered = pilot.read_text(encoding="utf-8")
        self.assertIn("blinded = true", rendered)
        self.assertNotIn("contract_delta_ids", rendered)
        rendered = rendered.replace('contract_label = ""', 'contract_label = "contract-present"', 1)
        rendered = rendered.replace(
            "expected_contract_ids = []",
            'expected_contract_ids = ["security-boundary/boundary-changed"]',
            1,
        )
        rendered = rendered.replace('rationale = ""', 'rationale = "reviewed the change"', 1)
        rendered = rendered.replace('contract_label = ""', 'contract_label = "no-contract"', 1)
        rendered = rendered.replace('rationale = ""', 'rationale = "reviewed the change"', 1)
        pilot.write_text(rendered, encoding="utf-8")
        result = validate_contract_pilot(
            pilot, self.collection, self.manifest, self.rules, self.zoo
        )
        self.assertEqual(result["status"], "valid")
        self.assertEqual(result["cases"], 2)

    def test_metrics_are_contract_scoped_and_exploratory(self) -> None:
        pilot = self.root / "pilot.toml"
        rendered = render_contract_pilot_template(
            self.collection, self.manifest, self.rules, self.zoo, "expert"
        )
        rendered = rendered.replace('contract_label = ""', 'contract_label = "contract-present"', 1)
        rendered = rendered.replace(
            "expected_contract_ids = []",
            'expected_contract_ids = ["security-boundary/boundary-changed"]',
            1,
        )
        rendered = rendered.replace('rationale = ""', 'rationale = "confirmed boundary change"', 1)
        rendered = rendered.replace('contract_label = ""', 'contract_label = "no-contract"', 1)
        rendered = rendered.replace('rationale = ""', 'rationale = "no measured contract"', 1)
        pilot.write_text(rendered, encoding="utf-8")
        result = build_contract_pilot_metrics(
            pilot, self.collection, self.manifest, self.rules, self.zoo
        )
        counts = result["contract_counts"]["security-boundary/boundary-changed"]
        self.assertEqual(counts, {"tp": 1, "fn": 0, "tn": 1, "fp": 0, "excluded": 0})
        self.assertEqual(result["scope"], "single-expert exploratory real-history contract pilot")
        self.assertIn("not independent validation", result["limitation"])
        self.assertEqual(result["metrics"]["case_coverage"]["value"], 1.0)

    def test_pilot_rejects_unmeasured_contract(self) -> None:
        pilot = self.root / "pilot.toml"
        rendered = render_contract_pilot_template(
            self.collection, self.manifest, self.rules, self.zoo, "expert"
        ).replace('contract_label = ""', 'contract_label = "contract-present"', 1)
        rendered = rendered.replace(
            "expected_contract_ids = []",
            'expected_contract_ids = ["delivery/action-reference-changed"]',
            1,
        )
        rendered = rendered.replace('rationale = ""', 'rationale = "unsupported"', 1)
        pilot.write_text(rendered, encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "unknown contract ID"):
            validate_contract_pilot(pilot, self.collection, self.manifest, self.rules, self.zoo)


if __name__ == "__main__":
    unittest.main()
