#!/usr/bin/env python3
"""Run and validate the controlled ChangeProof Benchmark v1."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from changeproof_benchmark_artifact import render_report, validate_artifact, validate_artifact_file
from changeproof_benchmark_contract import BenchmarkManifestError, PROTOCOL, validate_manifest
from changeproof_benchmark_proof import BenchmarkProofError
from changeproof_benchmark_runner import collect_benchmark


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs="?", choices=("check", "collect", "validate-result", "report"), default="check")
    parser.add_argument("--manifest", type=Path, default=Path("tests/benchmarks/changeproof.toml"))
    parser.add_argument("--fixture-root", type=Path, default=Path("tests/fixtures/review-zoo"))
    parser.add_argument("--artifact", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    parser.add_argument("--scanner")
    parser.add_argument("--timeout", type=int, default=60)
    parser.add_argument("--format", choices=("text", "json"), default="text")
    args = parser.parse_args(argv)
    if args.command == "collect" and args.output is None:
        print("--output is required for collect", file=sys.stderr)
        return 2
    if args.command in {"validate-result", "report"} and args.artifact is None:
        print("--artifact is required", file=sys.stderr)
        return 2
    if args.command == "report" and args.output is None:
        print("--output is required for report", file=sys.stderr)
        return 2
    if args.command == "collect" and args.timeout <= 0:
        print("--timeout must be positive", file=sys.stderr)
        return 2
    try:
        if args.command == "check":
            corpus, cases = validate_manifest(args.manifest, args.fixture_root)
            print(f"ChangeProof benchmark: {PROTOCOL} ({corpus}; {len(cases)} cases)")
            return 0
        if args.command == "collect":
            artifact = collect_benchmark(
                args.repo_root.resolve(), args.manifest, args.fixture_root, args.scanner, args.timeout
            )
            validate_artifact(artifact, args.manifest, args.fixture_root)
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(artifact, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            print(f"ChangeProof benchmark artifact: {args.output} ({len(artifact['cases'])} cases)")
            return 0
        validated = validate_artifact_file(args.artifact, args.manifest, args.fixture_root)
        if args.command == "validate-result":
            if args.format == "json":
                print(json.dumps(validated["summary"], indent=2, sort_keys=True))
            else:
                summary = validated["summary"]
                print(f"ChangeProof benchmark artifact: valid ({summary['case_count']} cases)")
            return 0
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(render_report(validated), encoding="utf-8")
        print(f"ChangeProof benchmark report: {args.output}")
        return 0
    except (BenchmarkManifestError, BenchmarkProofError, OSError, json.JSONDecodeError) as error:
        print(f"ChangeProof benchmark failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
