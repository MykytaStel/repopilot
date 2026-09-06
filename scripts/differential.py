#!/usr/bin/env python3
"""Validate the preregistered differential utility benchmark contract."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from differential_artifact import validate_artifact, validate_data
from differential_contract import DifferentialManifestError, validate_differential
from differential_runner import collect_differential
from real_history_contract import HoldoutManifestError


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs="?", choices=("check", "collect", "validate-result"), default="check")
    parser.add_argument("--manifest", type=Path, default=Path("tests/benchmarks/differential.toml"))
    parser.add_argument("--holdout-manifest", type=Path, default=Path("tests/benchmarks/manifest.toml"))
    parser.add_argument("--rules-reference", type=Path, default=Path("docs/rules-reference.md"))
    parser.add_argument("--zoo-manifest", type=Path, default=Path("tests/zoo/manifest.toml"))
    parser.add_argument("--format", choices=("text", "json"), default="text")
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    parser.add_argument("--scanner", help="use an existing repopilot binary instead of building the workspace")
    parser.add_argument("--allow-version-mismatch", action="store_true")
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--output", type=Path, help="write a differential artifact")
    parser.add_argument("--artifact", type=Path, help="differential artifact to validate")
    args = parser.parse_args(argv)
    try:
        result = validate_differential(args.manifest, args.holdout_manifest, args.rules_reference, args.zoo_manifest)
    except (DifferentialManifestError, OSError) as error:
        print(f"differential manifest invalid: {error}", file=sys.stderr)
        return 1
    if args.command == "validate-result":
        if args.artifact is None:
            print("--artifact is required for validate-result", file=sys.stderr)
            return 2
        try:
            artifact_result = validate_artifact(
                args.artifact,
                args.holdout_manifest,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
            )
        except (DifferentialManifestError, HoldoutManifestError, OSError) as error:
            print(f"differential artifact invalid: {error}", file=sys.stderr)
            return 1
        if args.format == "json":
            print(json.dumps(artifact_result, indent=2, sort_keys=True))
        else:
            print(f"Differential artifact: {artifact_result['status']} ({artifact_result['cases']} cases)")
        return 0
    if args.command == "collect":
        if args.output is None:
            print("--output is required for collect", file=sys.stderr)
            return 2
        if args.timeout <= 0:
            print("--timeout must be positive", file=sys.stderr)
            return 2
        try:
            artifact = collect_differential(
                args.repo_root.resolve(),
                args.holdout_manifest,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
                args.scanner,
                args.allow_version_mismatch,
                args.timeout,
            )
            validate_data(
                artifact,
                args.holdout_manifest,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
            )
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(artifact, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        except (DifferentialManifestError, HoldoutManifestError, OSError) as error:
            print(f"differential collection failed: {error}", file=sys.stderr)
            return 2
        print(f"Differential collection: {args.output} ({len(artifact['cases'])} cases)")
        return 0
    if args.format == "json":
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(f"Differential protocol: {result['status']} ({result['cases']} cases)")
        print(f"Measurements: {', '.join(result['measurements'])}")
        print(f"Repetitions: {result['repetitions']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
