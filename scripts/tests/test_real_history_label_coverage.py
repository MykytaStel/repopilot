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

from real_history_annotations import render_worksheet  # noqa: E402
from real_history_adjudication import render_adjudication_template  # noqa: E402
from real_history_contract_pilot import render_contract_pilot_template  # noqa: E402
from real_history_contracts import contract_evidence_hash  # noqa: E402
from real_history_label_coverage import (  # noqa: E402
    build_label_coverage,
    validate_label_coverage,
    write_label_coverage,
)
from real_history_label_coverage_report import render_label_coverage_report  # noqa: E402
from test_real_history_annotations import AnnotationWorkflowTests  # noqa: E402


class LabelCoverageTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fixture = AnnotationWorkflowTests()
        self.fixture.setUp()

    def tearDown(self) -> None:
        self.fixture.tearDown()

    def _add_observations(self) -> None:
        collection = json.loads(self.fixture.collection.read_text(encoding="utf-8"))
        measured = "security-boundary/boundary-changed"
        unmeasured = "delivery/action-reference-changed"
        for case, contract_ids in zip(collection["cases"], ([measured], [unmeasured])):
            review = case["review"]
            review["contract_delta_ids"] = contract_ids
            review["contract_delta_count"] = len(contract_ids)
            review["contract_evidence_sha256"] = contract_evidence_hash(contract_ids)
        self.fixture.collection.write_text(json.dumps(collection), encoding="utf-8")

    def test_pending_audit_exposes_unmeasured_and_unreviewed_observations(self) -> None:
        self._add_observations()
        result = build_label_coverage(
            self.fixture.collection, self.fixture.manifest, self.fixture.rules, self.fixture.zoo
        )

        self.assertEqual(result["label_source"], "pending")
        self.assertEqual(result["cases_labeled"], 0)
        self.assertEqual(result["unmeasured_observed_contract_ids"], ["delivery/action-reference-changed"])
        self.assertEqual(result["unreviewed_observations"], [
            {"case_id": "case-one", "contract_ids": ["security-boundary/boundary-changed"]},
        ])
        self.assertEqual(
            result["contract_coverage"]["security-boundary/boundary-changed"]["observed_cases"],
            1,
        )
        self.assertTrue(result["contract_coverage"]["delivery/action-reference-changed"]["unmeasured"])

    def test_dual_audit_reports_expected_and_observed_contract_gaps(self) -> None:
        self._add_observations()
        left = self.fixture.root / "a.toml"
        right = self.fixture.root / "b.toml"
        left.write_text(
            render_worksheet(self.fixture.collection, self.fixture.manifest, self.fixture.rules, self.fixture.zoo, "a"),
            encoding="utf-8",
        )
        right.write_text(
            render_worksheet(self.fixture.collection, self.fixture.manifest, self.fixture.rules, self.fixture.zoo, "b"),
            encoding="utf-8",
        )
        for path in (left, right):
            text = path.read_text(encoding="utf-8")
            text = text.replace('label = ""', 'label = "defect-present"', 1)
            text = text.replace('expected_rule_ids = []', 'expected_rule_ids = ["demo.rule"]', 1)
            text = text.replace(
                "expected_contract_ids = []",
                'expected_contract_ids = ["security-boundary/boundary-changed"]',
                1,
            )
            text = text.replace('rationale = ""', 'rationale = "reviewed"', 1)
            text = text.replace('label = ""', 'label = "no-defect"', 1)
            text = text.replace('rationale = ""', 'rationale = "reviewed"', 1)
            path.write_text(text, encoding="utf-8")
        adjudication = self.fixture.root / "adjudication.toml"
        adjudication.write_text(
            render_adjudication_template(
                left, right, self.fixture.collection, self.fixture.manifest, self.fixture.rules, self.fixture.zoo
            ).replace('adjudicated = ""', 'adjudicated = "defect-present"', 1)
            .replace('expected_rule_ids = []', 'expected_rule_ids = ["demo.rule"]', 1)
            .replace('expected_contract_ids = []', 'expected_contract_ids = ["security-boundary/boundary-changed"]', 1)
            .replace('rationale = ""', 'rationale = "confirmed"', 1)
            .replace('adjudicated = ""', 'adjudicated = "no-defect"', 1)
            .replace('rationale = ""', 'rationale = "confirmed"', 1),
            encoding="utf-8",
        )
        result = build_label_coverage(
            self.fixture.collection,
            self.fixture.manifest,
            self.fixture.rules,
            self.fixture.zoo,
            adjudication_path=adjudication,
            annotation_a_path=left,
            annotation_b_path=right,
        )

        self.assertEqual(result["label_source"], "dual-adjudication")
        self.assertEqual(result["cases_labeled"], 2)
        self.assertEqual(result["unreviewed_observations"], [])
        self.assertEqual(result["cases"][0]["unmeasured_observed_contract_ids"], [])
        self.assertIn("delivery/action-reference-changed", result["cases"][1]["unmeasured_observed_contract_ids"])

    def test_pilot_audit_is_explicitly_single_expert(self) -> None:
        pilot = self.fixture.root / "pilot.toml"
        pilot.write_text(
            render_contract_pilot_template(
                self.fixture.collection, self.fixture.manifest, self.fixture.rules, self.fixture.zoo, "expert"
            ),
            encoding="utf-8",
        )
        text = pilot.read_text(encoding="utf-8")
        text = text.replace('contract_label = ""', 'contract_label = "no-contract"')
        text = text.replace('rationale = ""', 'rationale = "reviewed"')
        pilot.write_text(text, encoding="utf-8")
        result = build_label_coverage(
            self.fixture.collection,
            self.fixture.manifest,
            self.fixture.rules,
            self.fixture.zoo,
            pilot_path=pilot,
        )

        self.assertEqual(result["label_source"], "single-expert-pilot")
        self.assertFalse(result["independent_validation"])
        self.assertEqual(result["cases_labeled"], 2)

    def test_report_is_deterministic_and_scope_explicit(self) -> None:
        result = build_label_coverage(
            self.fixture.collection, self.fixture.manifest, self.fixture.rules, self.fixture.zoo
        )
        report = render_label_coverage_report(result)
        self.assertEqual(report, render_label_coverage_report(result))
        self.assertIn("pending", report)
        self.assertIn("delivery/action-reference-changed", report)
        self.assertIn("unreviewed", report.lower())

    def test_coverage_validator_rejects_tampering(self) -> None:
        coverage = self.fixture.root / "coverage.json"
        write_label_coverage(
            coverage,
            self.fixture.collection,
            self.fixture.manifest,
            self.fixture.rules,
            self.fixture.zoo,
        )
        validated = validate_label_coverage(
            coverage,
            self.fixture.collection,
            self.fixture.manifest,
            self.fixture.rules,
            self.fixture.zoo,
        )
        self.assertEqual(validated["status"], "valid")
        data = json.loads(coverage.read_text(encoding="utf-8"))
        data["cases_labeled"] = 1
        coverage.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "coverage artifact does not match"):
            validate_label_coverage(
                coverage,
                self.fixture.collection,
                self.fixture.manifest,
                self.fixture.rules,
                self.fixture.zoo,
            )


if __name__ == "__main__":
    unittest.main()
