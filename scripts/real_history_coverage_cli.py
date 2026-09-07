"""CLI handlers for real-history label coverage audits."""

from __future__ import annotations

import json
import sys
from argparse import Namespace

from real_history_contract import HoldoutManifestError
from real_history_label_coverage import (
    build_label_coverage,
    validate_label_coverage,
    write_label_coverage,
)
from real_history_label_coverage_report import write_label_coverage_report


def handle_coverage_command(args: Namespace) -> int | None:
    if args.command == "label-coverage":
        if args.artifact is None:
            print("label-coverage requires --artifact", file=sys.stderr)
            return 2
        try:
            if args.output is None:
                result = build_label_coverage(
                    args.artifact,
                    args.manifest,
                    args.rules_reference,
                    args.zoo_manifest,
                    adjudication_path=args.annotation,
                    annotation_a_path=args.annotation_a,
                    annotation_b_path=args.annotation_b,
                    pilot_path=args.pilot,
                )
            else:
                result = write_label_coverage(
                    args.output,
                    args.artifact,
                    args.manifest,
                    args.rules_reference,
                    args.zoo_manifest,
                    adjudication_path=args.annotation,
                    annotation_a_path=args.annotation_a,
                    annotation_b_path=args.annotation_b,
                    pilot_path=args.pilot,
                )
        except (HoldoutManifestError, OSError) as error:
            print(f"label coverage failed: {error}", file=sys.stderr)
            return 1
        if args.output is not None:
            print(f"Label coverage: {args.output} ({result['label_source']})")
        elif args.format == "json":
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(
                f"Label coverage: {result['cases_labeled']}/{result['cases_total']} cases labeled; "
                f"source {result['label_source']}"
            )
            print(
                "Unmeasured observed contracts: "
                + (", ".join(result["unmeasured_observed_contract_ids"]) or "none")
            )
            print(f"Unreviewed measured observations: {len(result['unreviewed_observations'])}")
        return 0
    if args.command == "validate-label-coverage":
        if args.coverage is None or args.artifact is None:
            print("validate-label-coverage requires --coverage and --artifact", file=sys.stderr)
            return 2
        try:
            result = validate_label_coverage(
                args.coverage,
                args.artifact,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
                adjudication_path=args.annotation,
                annotation_a_path=args.annotation_a,
                annotation_b_path=args.annotation_b,
                pilot_path=args.pilot,
            )
        except (HoldoutManifestError, OSError) as error:
            print(f"label coverage invalid: {error}", file=sys.stderr)
            return 1
        print(f"Label coverage: valid ({result['cases']} cases; {result['label_source']})")
        return 0
    if args.command == "coverage-report":
        if args.coverage is None or args.output is None:
            print("coverage-report requires --coverage and --output", file=sys.stderr)
            return 2
        try:
            write_label_coverage_report(args.coverage, args.output)
        except (HoldoutManifestError, OSError, ValueError, json.JSONDecodeError) as error:
            print(f"coverage report failed: {error}", file=sys.stderr)
            return 1
        print(f"Label coverage report: {args.output}")
        return 0
    return None
