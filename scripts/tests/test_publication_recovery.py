from __future__ import annotations

import importlib.util
import json
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/release/publication-recovery.json"
SCRIPT = ROOT / "scripts/publication_state.py"
SPEC = importlib.util.spec_from_file_location("publication_state", SCRIPT)
assert SPEC and SPEC.loader
publication_state = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = publication_state
SPEC.loader.exec_module(publication_state)


class PublicationRecoveryFixtureTests(unittest.TestCase):
    def test_partial_second_run_only_reuses_matching_channels(self) -> None:
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        observations = []

        for channel in fixture["channels"]:
            observation = publication_state.Observation(
                channel=channel["channel"],
                expected_version=fixture["expected_version"],
                observed_version=channel.get("observed_version"),
                expected_digest=channel.get("expected_digest"),
                observed_digest=channel.get("observed_digest"),
                error_kind=channel.get("error_kind"),
            )
            observations.append(observation)
            self.assertEqual(publication_state.classify(observation).value, channel["state"])
            self.assertEqual(publication_state.action(observation), channel["action"])

        self.assertEqual(
            publication_state.aggregate(observations),
            publication_state.AggregateState.MISMATCHED,
        )

        matching = [
            observation
            for observation in observations
            if publication_state.action(observation) == "skip"
        ]
        incomplete = [
            observation
            for observation in observations
            if publication_state.action(observation) in {"publish", "retry"}
        ]
        self.assertEqual([item.channel for item in matching], ["github"])
        self.assertEqual(
            [item.channel for item in incomplete], ["npm-root", "crates"]
        )

    def test_mismatch_is_a_terminal_recovery_guard(self) -> None:
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        mismatch = next(
            item for item in fixture["channels"] if item["state"] == "published-mismatch"
        )
        observation = publication_state.Observation(
            channel=mismatch["channel"],
            expected_version=fixture["expected_version"],
            observed_version=mismatch["observed_version"],
            expected_digest=mismatch["expected_digest"],
            observed_digest=mismatch["observed_digest"],
        )

        self.assertEqual(publication_state.action(observation), "fail")
        self.assertNotIn(publication_state.action(observation), {"publish", "retry"})


if __name__ == "__main__":
    unittest.main()
