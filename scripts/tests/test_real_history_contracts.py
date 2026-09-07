from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from real_history_contracts import (  # noqa: E402
    CONTRACT_IDS,
    ALL_CONTRACT_IDS,
    ContractEvidenceError,
    contract_evidence_hash,
    observed_contract_ids,
    validate_expected_contract_ids,
)


class ContractEvidenceTests(unittest.TestCase):
    def test_normalizes_observed_contract_ids_and_hashes_only_stable_identity(self) -> None:
        report = {
            "change_proof": {
                "contract_deltas": [
                    {"family": "test-coverage", "change": "test-missing", "evidence": "run one"},
                    {"family": "security-boundary", "change": "boundary-changed", "evidence": "run two"},
                    {"family": "test-coverage", "change": "test-missing", "evidence": "run three"},
                ]
            }
        }

        ids = observed_contract_ids(report)

        self.assertEqual(
            ids,
            ("security-boundary/boundary-changed", "test-coverage/test-missing"),
        )
        self.assertEqual(len(contract_evidence_hash(ids)), 64)
        self.assertEqual(contract_evidence_hash(ids), contract_evidence_hash(tuple(reversed(ids))))

    def test_missing_change_proof_is_an_empty_observation(self) -> None:
        self.assertEqual(observed_contract_ids({}), ())

    def test_rejects_unknown_observed_contract(self) -> None:
        with self.assertRaisesRegex(ContractEvidenceError, "unknown contract ID"):
            observed_contract_ids(
                {"change_proof": {"contract_deltas": [{"family": "new", "change": "new"}]}}
            )

    def test_observed_registry_keeps_unmeasured_current_families_visible(self) -> None:
        self.assertIn("dependency/upgraded", ALL_CONTRACT_IDS)
        self.assertEqual(
            observed_contract_ids(
                {"change_proof": {"contract_deltas": [{"family": "dependency", "change": "upgraded"}]}}
            ),
            ("dependency/upgraded",),
        )

    def test_expected_ids_are_unique_and_protocol_bounded(self) -> None:
        self.assertEqual(
            validate_expected_contract_ids(
                ["test-coverage/test-missing", "security-boundary/boundary-changed"]
            ),
            ("security-boundary/boundary-changed", "test-coverage/test-missing"),
        )
        with self.assertRaisesRegex(ContractEvidenceError, "duplicate"):
            validate_expected_contract_ids(["test-coverage/test-missing"] * 2)
        with self.assertRaisesRegex(ContractEvidenceError, "unknown contract ID"):
            validate_expected_contract_ids(["security-boundary/not-supported"])

    def test_registry_is_explicit_and_sorted(self) -> None:
        self.assertEqual(
            CONTRACT_IDS,
            (
                "security-boundary/boundary-changed",
                "security-boundary/entrypoint-impacted",
                "test-coverage/test-changed",
                "test-coverage/test-missing",
            ),
        )


if __name__ == "__main__":
    unittest.main()
