from __future__ import annotations

import hashlib
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from real_history_annotations import (  # noqa: E402
    load_annotation,
    render_worksheet,
)
from real_history_adjudication import (  # noqa: E402
    render_adjudication_template,
    validate_adjudication,
)
from real_history_metrics import _case_outcome, build_metrics, wilson_interval  # noqa: E402
from real_history_contract import validate_manifest  # noqa: E402
from real_history_runner import BASELINE_COMMANDS  # noqa: E402


class AnnotationWorkflowTests(unittest.TestCase):
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
        _, _, cases = validate_manifest(self.manifest, self.rules, self.zoo)
        observations = []
        for case in cases:
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
                        "in_diff_rule_ids": ["demo.rule"] if case.case_id == "case-one" else [],
                    },
                }
            )
        data = {
            "schema_version": 1,
            "corpus": "test",
            "protocol": "dual-independent-adjudication-v1",
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
        self.collection.write_text(__import__("json").dumps(data), encoding="utf-8")

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def worksheet(self, reviewer: str) -> Path:
        path = self.root / f"{reviewer}.toml"
        path.write_text(
            render_worksheet(self.collection, self.manifest, self.rules, self.zoo, reviewer),
            encoding="utf-8",
        )
        return path

    def complete(self, path: Path, label: str, rationale: str = "reviewed diff") -> None:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
        for case in data["case"]:
            case["label"] = label
            case["rationale"] = rationale
            if label == "defect-present":
                case["expected_rule_ids"] = ["demo.rule"]
        lines = path.read_text(encoding="utf-8")
        for case in data["case"]:
            lines = lines.replace('label = ""', f'label = "{case["label"]}"', 1)
            lines = lines.replace('rationale = ""', f'rationale = "{case["rationale"]}"', 1)
            if label == "defect-present":
                lines = lines.replace("expected_rule_ids = []", 'expected_rule_ids = ["demo.rule"]', 1)
        path.write_text(lines, encoding="utf-8")

    def test_templates_are_deterministic_except_reviewer(self) -> None:
        left = render_worksheet(self.collection, self.manifest, self.rules, self.zoo, "a")
        right = render_worksheet(self.collection, self.manifest, self.rules, self.zoo, "b")
        self.assertEqual(left.replace('reviewer = "a"', 'reviewer = "b"'), right)
        self.assertEqual(tomllib.loads(left)["case"][0]["label"], "")

    def test_blank_template_is_not_admissible_as_annotation(self) -> None:
        path = self.worksheet("a")
        with self.assertRaisesRegex(ValueError, "label must be"):
            load_annotation(path, self.collection, self.manifest, self.rules, self.zoo, "a")

    def test_annotation_requires_pinned_context(self) -> None:
        path = self.worksheet("a")
        self.complete(path, "no-defect")
        text = path.read_text(encoding="utf-8").replace(
            'base_sha = "' + "a" * 40 + '"',
            'base_sha = "' + "b" * 40 + '"',
        )
        path.write_text(text, encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "base_sha does not match"):
            load_annotation(path, self.collection, self.manifest, self.rules, self.zoo, "a")

    def test_adjudication_template_preserves_disagreement(self) -> None:
        left, right = self.worksheet("a"), self.worksheet("b")
        self.complete(left, "defect-present")
        self.complete(right, "no-defect")
        rendered = render_adjudication_template(
            left, right, self.collection, self.manifest, self.rules, self.zoo
        )
        data = tomllib.loads(rendered)
        self.assertEqual(data["case"][0]["label_a"], "defect-present")
        self.assertEqual(data["case"][0]["label_b"], "no-defect")
        self.assertEqual(data["case"][0]["adjudicated"], "")

    def test_adjudication_requires_explicit_decision(self) -> None:
        left, right = self.worksheet("a"), self.worksheet("b")
        self.complete(left, "no-defect")
        self.complete(right, "no-defect")
        path = self.root / "adjudication.toml"
        path.write_text(
            render_adjudication_template(
                left, right, self.collection, self.manifest, self.rules, self.zoo
            ),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ValueError, "adjudicated label is required"):
            validate_adjudication(
                path, left, right, self.collection, self.manifest, self.rules, self.zoo
            )
        text = path.read_text(encoding="utf-8")
        path.write_text(
            text.replace('adjudicated = ""', 'adjudicated = "no-defect"').replace(
                'rationale = ""', 'rationale = "third reviewer confirmed"'
            ),
            encoding="utf-8",
        )
        self.assertEqual(
            validate_adjudication(path, left, right, self.collection, self.manifest, self.rules, self.zoo),
            2,
        )

    def test_metrics_are_corpus_scoped_and_include_intervals(self) -> None:
        left, right = self.worksheet("a"), self.worksheet("b")
        self.complete(left, "defect-present")
        self.complete(right, "defect-present")
        for path in (left, right):
            text = path.read_text(encoding="utf-8")
            marker = '[[case]]\nid = "case-two"'
            head, tail = text.split(marker, 1)
            tail = tail.replace('label = "defect-present"', 'label = "no-defect"', 1)
            tail = tail.replace('expected_rule_ids = ["demo.rule"]', "expected_rule_ids = []", 1)
            path.write_text(head + marker + tail, encoding="utf-8")
        adjudication = self.root / "adjudication.toml"
        adjudication.write_text(
            render_adjudication_template(
                left, right, self.collection, self.manifest, self.rules, self.zoo
            ),
            encoding="utf-8",
        )
        text = adjudication.read_text(encoding="utf-8")
        text = text.replace('adjudicated = ""', 'adjudicated = "defect-present"', 1)
        text = text.replace('expected_rule_ids = []', 'expected_rule_ids = ["demo.rule"]', 1)
        text = text.replace('rationale = ""', 'rationale = "confirmed"', 1)
        text = text.replace('adjudicated = ""', 'adjudicated = "no-defect"', 1)
        text = text.replace('rationale = ""', 'rationale = "confirmed"', 1)
        adjudication.write_text(text, encoding="utf-8")
        result = build_metrics(
            adjudication, left, right, self.collection, self.manifest, self.rules, self.zoo
        )
        self.assertEqual(result["counts"], {"tp": 1, "fn": 0, "tn": 1, "fp": 0, "excluded": 0})
        self.assertEqual(result["metrics"]["recall"]["trials"], 1)
        self.assertEqual(result["scope"], "adjudicated real-history holdout cases only")
        self.assertIsNotNone(result["metrics"]["recall"]["wilson_95"])
        self.assertIsNone(wilson_interval(0, 0))

    def test_case_outcomes_cover_confusion_matrix_and_uncertainty(self) -> None:
        self.assertEqual(_case_outcome("defect-present", ["demo.rule"], ["demo.rule"]), "tp")
        self.assertEqual(_case_outcome("defect-present", ["demo.rule"], []), "fn")
        self.assertEqual(_case_outcome("no-defect", [], []), "tn")
        self.assertEqual(_case_outcome("no-defect", [], ["demo.rule"]), "fp")
        self.assertEqual(_case_outcome("uncertain", [], ["demo.rule"]), "excluded")


if __name__ == "__main__":
    unittest.main()
