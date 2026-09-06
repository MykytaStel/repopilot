"""Validate case identity and baselines shared by both benchmark manifests."""

from __future__ import annotations

from differential_manifest import DifferentialCase, DifferentialManifestError


def validate_case_alignment(expected_cases: list[object], observed_cases: list[DifferentialCase]) -> None:
    expected = {case.case_id: case for case in expected_cases}
    observed = {case.case_id: case for case in observed_cases}
    if set(observed) != set(expected):
        missing = sorted(set(expected) - set(observed))
        extra = sorted(set(observed) - set(expected))
        raise DifferentialManifestError(f"case set mismatch (missing={missing}, extra={extra})")
    for case_id, case in observed.items():
        if tuple(sorted(case.baseline_ids)) != tuple(sorted(expected[case_id].baseline_ids)):
            raise DifferentialManifestError(f"case {case_id}: baseline set does not match holdout manifest")
