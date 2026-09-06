#!/usr/bin/env python3
"""Validate the preregistered differential utility benchmark contract."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from differential_contract import DifferentialManifestError, validate_differential


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs="?", choices=("check",), default="check")
    parser.add_argument("--manifest", type=Path, default=Path("tests/benchmarks/differential.toml"))
    parser.add_argument("--holdout-manifest", type=Path, default=Path("tests/benchmarks/manifest.toml"))
    parser.add_argument("--rules-reference", type=Path, default=Path("docs/rules-reference.md"))
    parser.add_argument("--zoo-manifest", type=Path, default=Path("tests/zoo/manifest.toml"))
    parser.add_argument("--format", choices=("text", "json"), default="text")
    args = parser.parse_args(argv)
    try:
        result = validate_differential(
            args.manifest, args.holdout_manifest, args.rules_reference, args.zoo_manifest
        )
    except (DifferentialManifestError, OSError) as error:
        print(f"differential manifest invalid: {error}", file=sys.stderr)
        return 1
    if args.format == "json":
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(f"Differential protocol: {result['status']} ({result['cases']} cases)")
        print(f"Measurements: {', '.join(result['measurements'])}")
        print(f"Repetitions: {result['repetitions']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
