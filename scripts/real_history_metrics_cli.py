"""CLI handlers for dual-review metrics integrity and reports."""

from __future__ import annotations

import json
import sys
from argparse import Namespace

from real_history_contract import HoldoutManifestError
from real_history_metrics import validate_metrics
from real_history_metrics_report import write_metrics_report


def handle_metrics_command(args: Namespace) -> int | None:
    if args.command == "validate-metrics":
        required = (args.metrics, args.annotation, args.annotation_a, args.annotation_b, args.artifact)
        if any(value is None for value in required):
            print(
                "validate-metrics requires --metrics, --artifact, --annotation-a, --annotation-b, and --annotation",
                file=sys.stderr,
            )
            return 2
        try:
            result = validate_metrics(
                args.metrics,
                args.annotation,
                args.annotation_a,
                args.annotation_b,
                args.artifact,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
            )
        except (HoldoutManifestError, OSError) as error:
            print(f"metrics invalid: {error}", file=sys.stderr)
            return 1
        print(f"Metrics: valid ({result['cases']} cases)")
        return 0
    if args.command == "metrics-report":
        if args.metrics is None or args.output is None:
            print("metrics-report requires --metrics and --output", file=sys.stderr)
            return 2
        try:
            write_metrics_report(args.metrics, args.output)
        except (OSError, ValueError, json.JSONDecodeError) as error:
            print(f"metrics report failed: {error}", file=sys.stderr)
            return 1
        print(f"Metrics report: {args.output}")
        return 0
    return None
