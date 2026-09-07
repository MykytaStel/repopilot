from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
TESTS_DIR = Path(__file__).resolve().parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))
if str(TESTS_DIR) not in sys.path:
    sys.path.insert(0, str(TESTS_DIR))

from real_history_adjudication import render_adjudication_template  # noqa: E402
from real_history_contracts import contract_evidence_hash  # noqa: E402
from real_history_metrics import validate_metrics, write_metrics  # noqa: E402
from real_history_metrics_report import render_metrics_report  # noqa: E402
from test_real_history_annotations import AnnotationWorkflowTests  # noqa: E402


class MetricsIntegrityTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fixture = AnnotationWorkflowTests()
        self.fixture.setUp()

    def tearDown(self) -> None:
        self.fixture.tearDown()

    def _adjudicated_inputs(self) -> tuple[Path, Path, Path]:
        fixture = self.fixture
        collection = json.loads(fixture.collection.read_text(encoding="utf-8"))
        collection["cases"][0]["review"]["contract_delta_ids"] = [
            "security-boundary/boundary-changed"
        ]
        collection["cases"][0]["review"]["contract_delta_count"] = 1
        collection["cases"][0]["review"]["contract_evidence_sha256"] = contract_evidence_hash(
            ("security-boundary/boundary-changed",)
        )
        fixture.collection.write_text(json.dumps(collection), encoding="utf-8")
        left, right = fixture.worksheet("a"), fixture.worksheet("b")
        fixture.complete(left, "defect-present")
        fixture.complete(right, "defect-present")
        for path in (left, right):
            text = path.read_text(encoding="utf-8").replace(
                "expected_contract_ids = []",
                'expected_contract_ids = ["security-boundary/boundary-changed"]',
                1,
            )
            marker = '[[case]]\nid = "case-two"'
            head, tail = text.split(marker, 1)
            tail = tail.replace('label = "defect-present"', 'label = "no-defect"', 1)
            tail = tail.replace('expected_rule_ids = ["demo.rule"]', "expected_rule_ids = []", 1)
            path.write_text(head + marker + tail, encoding="utf-8")
        adjudication = fixture.root / "adjudication.toml"
        adjudication.write_text(
            render_adjudication_template(
                left, right, fixture.collection, fixture.manifest, fixture.rules, fixture.zoo
            ),
            encoding="utf-8",
        )
        text = adjudication.read_text(encoding="utf-8")
        text = text.replace('adjudicated = ""', 'adjudicated = "defect-present"', 1)
        text = text.replace('expected_rule_ids = []', 'expected_rule_ids = ["demo.rule"]', 1)
        text = text.replace(
            "expected_contract_ids = []",
            'expected_contract_ids = ["security-boundary/boundary-changed"]',
            1,
        )
        text = text.replace('rationale = ""', 'rationale = "confirmed"', 1)
        text = text.replace('adjudicated = ""', 'adjudicated = "no-defect"', 1)
        text = text.replace('rationale = ""', 'rationale = "confirmed"', 1)
        adjudication.write_text(text, encoding="utf-8")
        return adjudication, left, right

    def test_metrics_artifact_validator_rejects_tampering(self) -> None:
        fixture = self.fixture
        adjudication, left, right = self._adjudicated_inputs()
        metrics = fixture.root / "metrics.json"
        write_metrics(
            metrics, adjudication, left, right, fixture.collection, fixture.manifest, fixture.rules, fixture.zoo
        )
        validated = validate_metrics(
            metrics, adjudication, left, right, fixture.collection, fixture.manifest, fixture.rules, fixture.zoo
        )
        self.assertEqual(validated["status"], "valid")
        tampered = json.loads(metrics.read_text(encoding="utf-8"))
        tampered["counts"]["tp"] += 1
        metrics.write_text(json.dumps(tampered), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "metrics artifact does not match"):
            validate_metrics(
                metrics, adjudication, left, right, fixture.collection, fixture.manifest, fixture.rules, fixture.zoo
            )

    def test_metrics_report_is_deterministic_and_scope_explicit(self) -> None:
        data = {
            "schema_version": 2,
            "corpus": "test",
            "protocol": "dual-independent-adjudication-v1",
            "scope": "adjudicated real-history holdout cases only",
            "limitation": "descriptive corpus evidence",
            "counts": {"tp": 1, "fn": 0, "tn": 1, "fp": 0, "excluded": 0},
            "contract_counts": {
                "security-boundary/boundary-changed": {
                    "tp": 1,
                    "fn": 0,
                    "tn": 1,
                    "fp": 0,
                    "excluded": 0,
                }
            },
            "contract_metrics": {
                "security-boundary/boundary-changed": {
                    "recall": {"value": 1.0},
                    "specificity": {"value": 1.0},
                    "precision": {"value": 1.0},
                }
            },
        }
        report = render_metrics_report(data)
        self.assertEqual(report, render_metrics_report(data))
        self.assertIn("adjudicated real-history holdout cases only", report)
        self.assertIn("security-boundary/boundary-changed", report)
        self.assertIn("descriptive corpus evidence", report)


if __name__ == "__main__":
    unittest.main()
