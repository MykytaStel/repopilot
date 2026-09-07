from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import review_contract_scorecard as rcs  # noqa: E402


def write_fixture(root: Path, relative: str, payload: dict) -> None:
    path = root / relative / "expected.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload), encoding="utf-8")


def contract(family: str, change: str, exporter: str = "src/auth/session.ts") -> dict[str, str]:
    return {
        "family": family,
        "change": change,
        "exporter_path": exporter,
        "consumer_path": exporter,
        "confidence": "limited",
    }


class DiscoverAndValidateTests(unittest.TestCase):
    def test_discovers_pairs_and_counts_contract_dimensions(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_fixture(root, "boundary/access-control/safe", {"description": "safe"})
            write_fixture(
                root,
                "boundary/access-control/unsafe",
                {
                    "description": "unsafe",
                    "expect": [{"family": "boundary", "kind": "boundary.access-control"}],
                    "contract_expect": [
                        contract("security-boundary", "boundary-changed"),
                        contract("test-coverage", "test-missing"),
                    ],
                },
            )
            write_fixture(root, "behavioral/network-call/safe", {"description": "safe"})
            write_fixture(
                root,
                "behavioral/network-call/unsafe",
                {"description": "unsafe", "expect": [{"family": "behavioral"}]},
            )

            score = rcs.collect_score(root)

        self.assertEqual(score.scenario_count, 2)
        self.assertEqual(score.safe_variant_count, 2)
        self.assertEqual(score.unsafe_variant_count, 2)
        self.assertEqual(score.contract_positive_fixture_count, 1)
        self.assertEqual(score.contract_expectation_count, 2)
        self.assertEqual(score.by_contract_family["security-boundary"].positive_fixtures, 1)
        self.assertEqual(
            score.by_contract_family["security-boundary"].changes,
            {"boundary-changed"},
        )
        self.assertEqual(score.by_contract_family["test-coverage"].positive_fixtures, 1)

    def test_rejects_unknown_contract_change(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_fixture(root, "boundary/access-control/safe", {"description": "safe"})
            write_fixture(
                root,
                "boundary/access-control/unsafe",
                {
                    "description": "unsafe",
                    "expect": [{}],
                    "contract_expect": [contract("security-boundary", "invented")],
                },
            )

            with self.assertRaisesRegex(rcs.ScorecardError, "unknown change"):
                rcs.collect_score(root)

    def test_requires_safe_and_unsafe_pair(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_fixture(
                root,
                "boundary/access-control/unsafe",
                {"description": "unsafe", "expect": [{}]},
            )

            with self.assertRaisesRegex(rcs.ScorecardError, "missing safe variant"):
                rcs.collect_score(root)

    def test_rejects_duplicate_contract_expectation(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_fixture(root, "boundary/access-control/safe", {"description": "safe"})
            item = contract("security-boundary", "boundary-changed")
            write_fixture(
                root,
                "boundary/access-control/unsafe",
                {"description": "unsafe", "expect": [{}], "contract_expect": [item, item]},
            )

            with self.assertRaisesRegex(rcs.ScorecardError, "duplicate contract expectation"):
                rcs.collect_score(root)

    def test_requires_evidence_for_each_promoted_contract_family(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_fixture(root, "boundary/access-control/safe", {"description": "safe"})
            write_fixture(
                root,
                "boundary/access-control/unsafe",
                {
                    "description": "unsafe",
                    "expect": [{}],
                    "contract_expect": [
                        contract("security-boundary", "boundary-changed"),
                    ],
                },
            )

            with self.assertRaisesRegex(rcs.ScorecardError, "test-coverage"):
                rcs.collect_score(root)


class RenderTests(unittest.TestCase):
    def test_render_is_deterministic_and_states_scope(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_fixture(root, "boundary/access-control/safe", {"description": "safe"})
            write_fixture(
                root,
                "boundary/access-control/unsafe",
                {
                    "description": "unsafe",
                    "expect": [{}],
                    "contract_expect": [
                        contract("security-boundary", "boundary-changed"),
                        contract("test-coverage", "test-missing"),
                    ],
                },
            )
            score = rcs.collect_score(root)

        first = rcs.render_scorecard(score)
        second = rcs.render_scorecard(score)
        self.assertEqual(first, second)
        self.assertIn("synthetic fixture evidence", first)
        self.assertIn("1 of 1 scenario pairs", first)
        self.assertIn("security-boundary", first)
        self.assertIn("does not establish real-repository precision or recall", first)


if __name__ == "__main__":
    unittest.main()
