#!/usr/bin/env python3
"""Classify immutable release-channel observations for safe recovery."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from enum import Enum


class State(str, Enum):
    ABSENT = "absent"
    MATCHING = "published-matching"
    MISMATCHED = "published-mismatch"
    AUTH_FAILURE = "auth-failure"
    NETWORK_FAILURE = "network-failure"
    RATE_LIMIT_FAILURE = "rate-limit-failure"
    SERVICE_FAILURE = "service-failure"


class AggregateState(str, Enum):
    COMPLETE = "complete"
    PARTIAL = "partial"
    MISMATCHED = "mismatched"
    BLOCKED = "blocked"


@dataclass(frozen=True)
class Observation:
    channel: str
    expected_version: str
    observed_version: str | None = None
    expected_digest: str | None = None
    observed_digest: str | None = None
    error_kind: str | None = None


def classify(observation: Observation) -> State:
    if observation.error_kind:
        failures = {
            "auth": State.AUTH_FAILURE,
            "network": State.NETWORK_FAILURE,
            "rate-limit": State.RATE_LIMIT_FAILURE,
            "service": State.SERVICE_FAILURE,
        }
        try:
            return failures[observation.error_kind]
        except KeyError as error:
            raise ValueError(f"unknown publication error: {observation.error_kind}") from error

    if observation.observed_version is None:
        return State.ABSENT
    if observation.observed_version != observation.expected_version:
        return State.MISMATCHED
    if (
        observation.expected_digest is not None
        and observation.observed_digest != observation.expected_digest
    ):
        return State.MISMATCHED
    return State.MATCHING


def action(observation: Observation) -> str:
    state = classify(observation)
    if state is State.MATCHING:
        return "skip"
    if state is State.ABSENT:
        return "publish"
    if state is State.MISMATCHED:
        return "fail"
    return "retry"


def aggregate(observations: list[Observation]) -> AggregateState:
    if not observations:
        return AggregateState.PARTIAL
    states = {classify(observation) for observation in observations}
    if State.MISMATCHED in states:
        return AggregateState.MISMATCHED
    if states & {
        State.AUTH_FAILURE,
        State.NETWORK_FAILURE,
        State.RATE_LIMIT_FAILURE,
        State.SERVICE_FAILURE,
    }:
        return AggregateState.BLOCKED
    if State.ABSENT in states:
        return AggregateState.PARTIAL
    return AggregateState.COMPLETE


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    classify_parser = subcommands.add_parser("classify")
    classify_parser.add_argument("--channel", required=True)
    classify_parser.add_argument("--expected-version", required=True)
    classify_parser.add_argument("--observed-version")
    classify_parser.add_argument("--expected-digest")
    classify_parser.add_argument("--observed-digest")
    classify_parser.add_argument("--error-kind")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    observation = Observation(
        channel=args.channel,
        expected_version=args.expected_version,
        observed_version=args.observed_version,
        expected_digest=args.expected_digest,
        observed_digest=args.observed_digest,
        error_kind=args.error_kind,
    )
    print(
        json.dumps(
            {
                "channel": observation.channel,
                "state": classify(observation).value,
                "action": action(observation),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
