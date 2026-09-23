from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from changeproof_benchmark_contract import BenchmarkCase  # noqa: E402
from changeproof_benchmark_proof import (  # noqa: E402
    BenchmarkProofError,
    evaluate_case,
    evaluate_normalized_proof,
    normalized_proof_sha256,
    proof_semantic_sha256,
)


def proof(*, reasons: list[dict[str, object]] | None = None, status: str = "assessed") -> dict[str, object]:
    return {
        "verdict": "REVIEW",
        "reasons": reasons or [{"code": "maybe-sensitive", "count": 1, "message": "host-specific prose"}],
        "coverage": {
            "scope": "changed",
            "requested_files": 1,
            "analyzed_files": 1,
            "excluded_files": 0,
            "unsupported_files": 0,
        },
        "obligations": {
            "applicable": 1,
            "satisfied": 0,
            "failed": 0,
            "unavailable": 0,
            "unselected": 1,
            "stale": 0,
        },
        "contract_deltas": [
            {"family": "security-boundary", "change": "access-control-changed", "confidence": "high", "path": "/tmp/project/auth.ts"}
        ],
        "capability_coverage": [
            {"id": "contract-deltas", "status": status, "count": 1, "message": "machine-specific wording"}
        ],
    }


def case(*, claims_state: str = "unknown", reason_codes_state: str = "known") -> BenchmarkCase:
    return BenchmarkCase(
        case_id="proof-case",
        fixture="boundary/access-control",
        variant="unsafe",
        split="evaluation",
        profile="default",
        mutation_kind="authorization",
        expected_verdict="REVIEW",
        claims_state=claims_state,
        claims=() if claims_state == "known-empty" else None,
        reason_codes_state=reason_codes_state,
        reason_codes=("maybe-sensitive",) if reason_codes_state == "known" else None,
        contracts_state="known",
        contracts=("security-boundary/access-control-changed",),
        capabilities_state="known",
        capabilities=("contract-deltas/assessed",),
        coverage_state="known",
        coverage={"scope": "changed", "requested_files": 1, "analyzed_files": 1, "excluded_files": 0, "unsupported_files": 0},
        obligations_state="unknown",
        obligations=None,
        oracle="mutation-rationale-v1",
        oracle_state="known",
    )


class ChangeProofBenchmarkProofTests(unittest.TestCase):
    def test_semantic_hash_ignores_reason_order_and_prose(self) -> None:
        left = proof(reasons=[{"code": "maybe-sensitive", "count": 1, "message": "first"}, {"code": "scope-coverage-incomplete", "count": 2, "message": "left"}])
        right = proof(reasons=[{"code": "scope-coverage-incomplete", "count": 2, "message": "right"}, {"code": "maybe-sensitive", "count": 1, "message": "second"}])

        self.assertEqual(proof_semantic_sha256(left), proof_semantic_sha256(right))

    def test_semantic_hash_changes_when_capability_becomes_limited(self) -> None:
        self.assertNotEqual(proof_semantic_sha256(proof(status="assessed")), proof_semantic_sha256(proof(status="limited")))

    def test_normalized_projection_reuses_the_same_semantic_hash(self) -> None:
        from changeproof_benchmark_proof import normalize_change_proof

        raw = proof()
        self.assertEqual(proof_semantic_sha256(raw), normalized_proof_sha256(normalize_change_proof(raw)))

    def test_missing_change_proof_is_rejected(self) -> None:
        with self.assertRaisesRegex(BenchmarkProofError, "change_proof must be an object"):
            evaluate_case(case(), {})

    def test_evaluation_reports_expected_and_unknown_dimensions_separately(self) -> None:
        evaluation = evaluate_case(case(), {"change_proof": proof()})

        self.assertEqual(evaluation["decision"]["status"], "pass")
        self.assertEqual(evaluation["reasons"]["status"], "pass")
        self.assertEqual(evaluation["claims"], {"status": "unavailable", "reason": "v1 has no observed claims projection"})

    def test_claims_remain_unavailable_without_an_observed_projection(self) -> None:
        evaluation = evaluate_case(case(claims_state="known-empty"), {"change_proof": proof()})

        self.assertEqual(evaluation["claims"], {"status": "unavailable", "reason": "v1 has no observed claims projection"})

    def test_normalized_proof_is_sufficient_for_repeatable_evaluation(self) -> None:
        normalized = {
            "verdict": "REVIEW",
            "reasons": [{"code": "maybe-sensitive", "count": 1}],
            "coverage": {"scope": "changed", "requested_files": 1, "analyzed_files": 1, "excluded_files": 0, "unsupported_files": 0},
            "obligations": {"applicable": 0, "satisfied": 0, "failed": 0, "unavailable": 0, "unselected": 1, "stale": 0},
            "contracts": [{"family": "security-boundary", "change": "access-control-changed", "confidence": "high"}],
            "capabilities": [{"id": "contract-deltas", "status": "assessed", "count": 1}],
        }

        self.assertEqual(evaluate_normalized_proof(case(), normalized)["decision"]["status"], "pass")

    def test_persisted_projection_rejects_extra_or_missing_fields(self) -> None:
        normalized = {"verdict": "REVIEW", "reasons": [], "coverage": {"scope": "changed", "requested_files": 0, "analyzed_files": 0, "excluded_files": 0, "unsupported_files": 0}, "obligations": {"applicable": 0, "satisfied": 0, "failed": 0, "unavailable": 0, "unselected": 0, "stale": 0}, "contracts": [], "capabilities": [], "raw_source": "/secret"}
        with self.assertRaisesRegex(BenchmarkProofError, "fields are invalid"):
            normalized_proof_sha256(normalized)

    def test_semantic_hash_retains_count_and_confidence(self) -> None:
        changed_count = proof(reasons=[{"code": "maybe-sensitive", "count": 2, "message": "prose"}])
        changed_confidence = proof()
        changed_confidence["contract_deltas"][0]["confidence"] = "limited"
        self.assertNotEqual(proof_semantic_sha256(proof()), proof_semantic_sha256(changed_count))
        self.assertNotEqual(proof_semantic_sha256(proof()), proof_semantic_sha256(changed_confidence))

    def test_projection_rejects_source_like_scope(self) -> None:
        raw = proof()
        raw["coverage"]["scope"] = "SECRET SOURCE\n/private/path"
        with self.assertRaisesRegex(BenchmarkProofError, "coverage.scope is invalid"):
            proof_semantic_sha256(raw)


if __name__ == "__main__":
    unittest.main()
