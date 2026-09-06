#!/usr/bin/env python3
"""Validate and collect the independent real-history holdout protocol."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from real_history_contract import (
    HoldoutManifestError,
    summary,
    validate_manifest,
)
from real_history_runner import collect_holdout


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs="?", choices=("check", "collect"), default="check")
    parser.add_argument("--manifest", type=Path, default=Path("tests/benchmarks/manifest.toml"))
    parser.add_argument("--rules-reference", type=Path, default=Path("docs/rules-reference.md"))
    parser.add_argument("--zoo-manifest", type=Path, default=Path("tests/zoo/manifest.toml"))
    parser.add_argument("--format", choices=("text", "json"), default="text")
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    parser.add_argument("--scanner", help="use an existing repopilot binary instead of building the workspace")
    parser.add_argument("--allow-version-mismatch", action="store_true")
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--output", type=Path, help="write a collection artifact instead of stdout")
    args = parser.parse_args(argv)
    try:
        corpus, protocol, cases = validate_manifest(args.manifest, args.rules_reference, args.zoo_manifest)
    except (HoldoutManifestError, OSError) as error:
        print(f"holdout manifest invalid: {error}", file=sys.stderr)
        return 1
    if args.command == "collect":
        if args.timeout <= 0:
            print("--timeout must be positive", file=sys.stderr)
            return 2
        try:
            data = collect_holdout(
                args.repo_root.resolve(),
                args.manifest,
                corpus,
                protocol,
                cases,
                args.scanner,
                args.allow_version_mismatch,
                args.timeout,
            )
        except (HoldoutManifestError, OSError) as error:
            print(f"holdout collection failed: {error}", file=sys.stderr)
            return 2
        rendered = json.dumps(data, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.write_text(rendered, encoding="utf-8")
        elif args.format == "json":
            print(rendered, end="")
        else:
            print(f"Holdout collection: {data['corpus']}")
            print(f"Cases collected: {len(data['cases'])}; labels: {data['label_state']}")
        return 0
    data = summary(corpus, protocol, cases)
    if args.format == "json":
        print(json.dumps(data, indent=2, sort_keys=True))
    else:
        print(f"Holdout corpus: {data['corpus']} ({data['protocol']})")
        print(f"Cases: {data['cases']} across {len(data['repositories'])} repositories")
        print(f"Label states: {data['label_states']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
