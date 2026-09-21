"""CLI handlers for the single-expert real-history contract pilot."""

from __future__ import annotations

import sys
from argparse import Namespace

from real_history_contract import HoldoutManifestError
from real_history_contract_pilot import render_contract_pilot_template, validate_contract_pilot
from real_history_contract_pilot_metrics import (
    validate_contract_pilot_metrics,
    write_contract_pilot_metrics,
)


def handle_contract_pilot_command(args: Namespace) -> int | None:
    if args.command == "contract-pilot-template":
        if args.artifact is None or args.output is None:
            print("contract-pilot-template requires --artifact and --output", file=sys.stderr)
            return 2
        try:
            rendered = render_contract_pilot_template(
                args.artifact,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
                args.pilot_reviewer,
            )
        except (HoldoutManifestError, OSError) as error:
            print(f"contract pilot template failed: {error}", file=sys.stderr)
            return 1
        args.output.write_text(rendered, encoding="utf-8")
        print(f"Contract pilot worksheet: {args.output}")
        return 0
    if args.command == "validate-contract-pilot":
        if args.artifact is None or args.pilot is None:
            print("validate-contract-pilot requires --artifact and --pilot", file=sys.stderr)
            return 2
        try:
            result = validate_contract_pilot(
                args.pilot,
                args.artifact,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
            )
        except (HoldoutManifestError, OSError) as error:
            print(f"contract pilot invalid: {error}", file=sys.stderr)
            return 1
        print(f"Contract pilot: valid ({result['cases']} cases; reviewer {result['reviewer']})")
        return 0
    if args.command == "contract-pilot-metrics":
        if args.artifact is None or args.pilot is None or args.output is None:
            print("contract-pilot-metrics requires --artifact, --pilot, and --output", file=sys.stderr)
            return 2
        try:
            result = write_contract_pilot_metrics(
                args.output,
                args.pilot,
                args.artifact,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
            )
        except (HoldoutManifestError, OSError) as error:
            print(f"contract pilot metrics failed: {error}", file=sys.stderr)
            return 1
        counts = result["counts"]
        print(
            f"Contract pilot metrics: {args.output} "
            f"(TP {counts['tp']}, FN {counts['fn']}, TN {counts['tn']}, FP {counts['fp']})"
        )
        return 0
    if args.command == "validate-contract-pilot-metrics":
        if args.artifact is None or args.pilot is None or args.metrics is None:
            print("validate-contract-pilot-metrics requires --artifact, --pilot, and --metrics", file=sys.stderr)
            return 2
        try:
            result = validate_contract_pilot_metrics(
                args.metrics,
                args.pilot,
                args.artifact,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
            )
        except (HoldoutManifestError, OSError) as error:
            print(f"contract pilot metrics invalid: {error}", file=sys.stderr)
            return 1
        print(f"Contract pilot metrics: valid ({result['cases']} cases; reviewer {result['reviewer']})")
        return 0
    return None
