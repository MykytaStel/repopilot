#!/usr/bin/env python3
"""Validate or run RepoPilot real-project sandbox cases."""

from __future__ import annotations

import argparse
import json
import shlex
import sys
from pathlib import Path

from sandbox_contract import (
    SandboxManifestError,
    load_manifest,
    validate_artifact,
    validate_pilot_summary,
)
from sandbox_pilot import run_pilot
from sandbox_runner import run_case


def _manifest_path(value: str) -> Path:
    return Path(value).expanduser()


def _check(args: argparse.Namespace) -> int:
    manifest = load_manifest(args.manifest)
    result = {
        "status": "valid",
        "schema_version": manifest.schema_version,
        "protocol": manifest.protocol,
        "corpus": manifest.corpus,
        "manifest_sha256": manifest.sha256,
        "projects": len(manifest.projects),
        "cases": len(manifest.cases),
        "policy_id": manifest.policy.policy_id,
    }
    print(
        json.dumps(result, indent=2, sort_keys=True)
        if args.format == "json"
        else f"Sandbox manifest: valid ({len(manifest.cases)} cases)"
    )
    return 0


def _run(args: argparse.Namespace) -> int:
    scanner = tuple(shlex.split(args.scanner)) if args.scanner else None
    result = run_case(
        args.manifest,
        args.case,
        args.output,
        source_override=args.source.resolve() if args.source else None,
        scanner=scanner,
        dry_run=args.dry_run,
        work_root=args.work_root,
    )
    print(
        json.dumps(result, indent=2, sort_keys=True)
        if args.format == "json"
        else f"Sandbox case: {result['status']} ({args.output})"
    )
    return 0 if result["status"] in {"passed", "dry-run"} else 1


def _validate(args: argparse.Namespace) -> int:
    result = validate_artifact(args.artifact, args.manifest)
    print(
        json.dumps(result, indent=2, sort_keys=True)
        if args.format == "json"
        else f"Sandbox artifact: valid ({result['case_id']})"
    )
    return 0


def _pilot(args: argparse.Namespace) -> int:
    scanner = tuple(shlex.split(args.scanner)) if args.scanner else None
    result = run_pilot(
        args.manifest,
        args.output,
        source_root=args.source_root,
        scanner=scanner,
        repeats=args.repeats,
        work_root=args.work_root,
    )
    print(
        json.dumps(result, indent=2, sort_keys=True)
        if args.format == "json"
        else f"Sandbox pilot: {result['status']} ({args.output})"
    )
    return 0 if result["status"] == "passed" else 1


def _validate_pilot(args: argparse.Namespace) -> int:
    result = validate_pilot_summary(args.artifact, args.manifest)
    print(
        json.dumps(result, indent=2, sort_keys=True)
        if args.format == "json"
        else f"Sandbox pilot summary: valid ({args.artifact})"
    )
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command",
        choices=("check", "run", "pilot", "validate-artifact", "validate-pilot"),
        default="check",
        nargs="?",
    )
    parser.add_argument(
        "--manifest",
        type=_manifest_path,
        default=Path(".zoo/repopilot-validation/manifest.toml"),
    )
    parser.add_argument("--format", choices=("text", "json"), default="text")
    parser.add_argument("--case")
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--source", type=Path, help="pinned local checkout used instead of cloning"
    )
    parser.add_argument(
        "--scanner", help="static scanner command, parsed without a shell"
    )
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--work-root", type=Path)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--artifact", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == "check":
            return _check(args)
        if args.command == "run":
            if args.case is None or args.output is None:
                parser.error("run requires --case and --output")
            return _run(args)
        if args.command == "pilot":
            if args.output is None:
                parser.error("pilot requires --output")
            return _pilot(args)
        if args.command == "validate-pilot":
            if args.artifact is None:
                parser.error("validate-pilot requires --artifact")
            return _validate_pilot(args)
        if args.artifact is None:
            parser.error("validate-artifact requires --artifact")
        return _validate(args)
    except SandboxManifestError as error:
        print(f"sandbox contract invalid: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
