from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "publication_state.py"
SPEC = importlib.util.spec_from_file_location("publication_state", SCRIPT)
assert SPEC and SPEC.loader
publication_state = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = publication_state
SPEC.loader.exec_module(publication_state)


class PublicationStateTests(unittest.TestCase):
    def test_matching_identity_is_safe_to_skip(self) -> None:
        observation = publication_state.Observation(
            channel="npm-root",
            expected_version="0.23.0",
            observed_version="0.23.0",
            expected_digest="sha512-local",
            observed_digest="sha512-local",
        )

        self.assertEqual(
            publication_state.classify(observation),
            publication_state.State.MATCHING,
        )
        self.assertEqual(publication_state.action(observation), "skip")

    def test_absent_identity_is_publishable(self) -> None:
        observation = publication_state.Observation(
            channel="npm-root",
            expected_version="0.23.0",
        )

        self.assertEqual(
            publication_state.classify(observation),
            publication_state.State.ABSENT,
        )
        self.assertEqual(publication_state.action(observation), "publish")

    def test_version_or_digest_mismatch_fails_closed(self) -> None:
        for observed_version, observed_digest in [
            ("0.22.0", "sha512-local"),
            ("0.23.0", "sha512-other"),
        ]:
            with self.subTest(observed_version=observed_version, observed_digest=observed_digest):
                observation = publication_state.Observation(
                    channel="npm-root",
                    expected_version="0.23.0",
                    observed_version=observed_version,
                    expected_digest="sha512-local",
                    observed_digest=observed_digest,
                )
                self.assertEqual(
                    publication_state.classify(observation),
                    publication_state.State.MISMATCHED,
                )
                self.assertEqual(publication_state.action(observation), "fail")

    def test_auth_network_and_service_failures_stay_distinct(self) -> None:
        for error_kind, expected in [
            ("auth", publication_state.State.AUTH_FAILURE),
            ("network", publication_state.State.NETWORK_FAILURE),
            ("rate-limit", publication_state.State.RATE_LIMIT_FAILURE),
            ("service", publication_state.State.SERVICE_FAILURE),
        ]:
            with self.subTest(error_kind):
                observation = publication_state.Observation(
                    channel="crates",
                    expected_version="0.23.0",
                    error_kind=error_kind,
                )
                self.assertEqual(publication_state.classify(observation), expected)
                self.assertEqual(publication_state.action(observation), "retry")

    def test_aggregate_marks_partial_and_mismatch_recovery(self) -> None:
        matching = publication_state.Observation(
            channel="github",
            expected_version="0.23.0",
            observed_version="0.23.0",
        )
        absent = publication_state.Observation(
            channel="npm-root",
            expected_version="0.23.0",
        )
        self.assertEqual(
            publication_state.aggregate([matching, absent]),
            publication_state.AggregateState.PARTIAL,
        )

        mismatch = publication_state.Observation(
            channel="crates",
            expected_version="0.23.0",
            observed_version="0.22.0",
        )
        self.assertEqual(
            publication_state.aggregate([matching, mismatch]),
            publication_state.AggregateState.MISMATCHED,
        )

    def test_empty_observations_are_not_complete(self) -> None:
        self.assertEqual(
            publication_state.aggregate([]),
            publication_state.AggregateState.PARTIAL,
        )

    def test_rate_limit_blocks_aggregate_convergence(self) -> None:
        observation = publication_state.Observation(
            channel="npm-root",
            expected_version="0.23.0",
            error_kind="rate-limit",
        )
        self.assertEqual(
            publication_state.aggregate([observation]),
            publication_state.AggregateState.BLOCKED,
        )

    def test_cli_emits_machine_readable_state_and_action(self) -> None:
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "classify",
                "--channel",
                "npm-root",
                "--expected-version",
                "0.23.0",
                "--observed-version",
                "0.23.0",
                "--expected-digest",
                "sha512-local",
                "--observed-digest",
                "sha512-local",
            ],
            check=True,
            capture_output=True,
            text=True,
        )

        self.assertEqual(
            json.loads(result.stdout),
            {
                "channel": "npm-root",
                "state": "published-matching",
                "action": "skip",
            },
        )


if __name__ == "__main__":
    unittest.main()
