#!/usr/bin/env python3
"""Validate and run RepoPilot's held-out recall corpus contract."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from recall_contract import (
    RecallManifestError,
    summary,
    validate_manifest,
)
from recall_runner import evaluate_case_report, run_corpus


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs="?", choices=("check", "run"), default="check")
    parser.add_argument("--manifest", type=Path, default=Path("tests/recall/manifest.toml"))
    parser.add_argument("--rules-reference", type=Path, default=Path("docs/rules-reference.md"))
    parser.add_argument("--fixture-root", type=Path, default=Path("tests/fixtures/recall"))
    parser.add_argument("--format", choices=("text", "json"), default="text")
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    parser.add_argument("--scanner", help="use an existing repopilot binary instead of building the workspace")
    parser.add_argument("--allow-version-mismatch", action="store_true")
    parser.add_argument("--output", type=Path, help="write a run artifact instead of stdout")
    args = parser.parse_args(argv)
    try:
        corpus, cases = validate_manifest(args.manifest, args.rules_reference, args.fixture_root)
    except RecallManifestError as error:
        print(f"recall manifest invalid: {error}", file=sys.stderr)
        return 1
    if args.command == "run":
        try:
            data = run_corpus(
                args.repo_root.resolve(),
                args.manifest,
                args.rules_reference,
                corpus,
                cases,
                args.scanner,
                args.allow_version_mismatch,
            )
        except RecallManifestError as error:
            print(f"recall run failed: {error}", file=sys.stderr)
            return 2
        rendered = json.dumps(data, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.write_text(rendered, encoding="utf-8")
        elif args.format == "json":
            print(rendered, end="")
        else:
            print(f"Recall run: {data['corpus']} ({data['status']})")
            print(f"Cases: {data['counts']['passed']}/{data['counts']['total']} passed")
        return 0 if data["status"] == "pass" else 1

    data = summary(corpus, cases)
    if args.format == "json":
        print(json.dumps(data, indent=2, sort_keys=True))
    else:
        print(f"Recall corpus: {data['corpus']}")
        print(f"Cases: {data['cases']} ({data['seeded_defects']} seeded defects, {data['safe_guards']} safe guards)")
        print(f"Rules: {', '.join(data['rules'])}")
        print(f"Languages: {', '.join(data['languages'])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
