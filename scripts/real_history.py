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
from real_history_artifact import validate_collection_artifact, validate_collection_data
from real_history_annotations import (
    HoldoutManifestError as AnnotationManifestError,
    load_annotation,
    render_worksheet,
)
from real_history_adjudication import render_adjudication_template, validate_adjudication
from real_history_contract_pilot import render_contract_pilot_template, validate_contract_pilot
from real_history_contract_pilot_metrics import build_contract_pilot_metrics
from real_history_metrics import write_metrics
from real_history_runner import collect_holdout


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command",
        nargs="?",
        choices=(
            "check",
            "collect",
            "validate-result",
            "template",
            "validate-annotation",
            "adjudication-template",
            "validate-adjudication",
            "metrics",
            "contract-pilot-template",
            "validate-contract-pilot",
            "contract-pilot-metrics",
        ),
        default="check",
    )
    parser.add_argument("--manifest", type=Path, default=Path("tests/benchmarks/manifest.toml"))
    parser.add_argument("--rules-reference", type=Path, default=Path("docs/rules-reference.md"))
    parser.add_argument("--zoo-manifest", type=Path, default=Path("tests/zoo/manifest.toml"))
    parser.add_argument("--format", choices=("text", "json"), default="text")
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    parser.add_argument("--scanner", help="use an existing repopilot binary instead of building the workspace")
    parser.add_argument("--allow-version-mismatch", action="store_true")
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--output", type=Path, help="write a collection artifact instead of stdout")
    parser.add_argument("--artifact", type=Path, help="collection artifact to validate")
    parser.add_argument("--annotation", type=Path, help="completed reviewer worksheet to validate")
    parser.add_argument("--annotation-a", type=Path, help="reviewer A worksheet")
    parser.add_argument("--annotation-b", type=Path, help="reviewer B worksheet")
    parser.add_argument("--reviewer", choices=("a", "b"), help="independent worksheet owner")
    parser.add_argument("--pilot", type=Path, help="single-expert contract pilot worksheet")
    parser.add_argument("--pilot-reviewer", default="expert", help="single-expert pilot reviewer label")
    args = parser.parse_args(argv)
    try:
        corpus, protocol, cases = validate_manifest(args.manifest, args.rules_reference, args.zoo_manifest)
    except (HoldoutManifestError, OSError) as error:
        print(f"holdout manifest invalid: {error}", file=sys.stderr)
        return 1
    if args.command == "validate-result":
        if args.artifact is None:
            print("--artifact is required for validate-result", file=sys.stderr)
            return 2
        try:
            result = validate_collection_artifact(
                args.artifact, args.manifest, args.rules_reference, args.zoo_manifest
            )
        except (HoldoutManifestError, OSError) as error:
            print(f"collection artifact invalid: {error}", file=sys.stderr)
            return 1
        if args.format == "json":
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"Collection artifact: {result['status']} ({result['cases']} cases)")
        return 0
    if args.command == "template":
        if args.artifact is None or args.output is None or args.reviewer is None:
            print("template requires --artifact, --reviewer, and --output", file=sys.stderr)
            return 2
        try:
            rendered = render_worksheet(
                args.artifact,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
                args.reviewer,
            )
        except (AnnotationManifestError, OSError) as error:
            print(f"annotation template failed: {error}", file=sys.stderr)
            return 1
        args.output.write_text(rendered, encoding="utf-8")
        print(f"Annotation worksheet: {args.output} (reviewer {args.reviewer})")
        return 0
    if args.command == "validate-annotation":
        if args.artifact is None or args.annotation is None or args.reviewer is None:
            print("validate-annotation requires --artifact, --annotation, and --reviewer", file=sys.stderr)
            return 2
        try:
            _, cases = load_annotation(
                args.annotation,
                args.artifact,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
                args.reviewer,
            )
        except (AnnotationManifestError, OSError) as error:
            print(f"annotation invalid: {error}", file=sys.stderr)
            return 1
        print(f"Annotation: valid ({len(cases)} cases; reviewer {args.reviewer})")
        return 0
    if args.command == "adjudication-template":
        required = (args.artifact, args.annotation_a, args.annotation_b, args.output)
        if any(value is None for value in required):
            print(
                "adjudication-template requires --artifact, --annotation-a, --annotation-b, and --output",
                file=sys.stderr,
            )
            return 2
        try:
            rendered = render_adjudication_template(
                args.annotation_a,
                args.annotation_b,
                args.artifact,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
            )
        except (AnnotationManifestError, OSError) as error:
            print(f"adjudication template failed: {error}", file=sys.stderr)
            return 1
        args.output.write_text(rendered, encoding="utf-8")
        print(f"Adjudication template: {args.output}")
        return 0
    if args.command == "validate-adjudication":
        required = (args.artifact, args.annotation_a, args.annotation_b, args.annotation)
        if any(value is None for value in required):
            print(
                "validate-adjudication requires --artifact, --annotation-a, --annotation-b, and --annotation",
                file=sys.stderr,
            )
            return 2
        try:
            count = validate_adjudication(
                args.annotation,
                args.annotation_a,
                args.annotation_b,
                args.artifact,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
            )
        except (AnnotationManifestError, OSError) as error:
            print(f"adjudication invalid: {error}", file=sys.stderr)
            return 1
        print(f"Adjudication: valid ({count} cases)")
        return 0
    if args.command == "metrics":
        required = (args.artifact, args.annotation_a, args.annotation_b, args.annotation, args.output)
        if any(value is None for value in required):
            print(
                "metrics requires --artifact, --annotation-a, --annotation-b, --annotation, and --output",
                file=sys.stderr,
            )
            return 2
        try:
            result = write_metrics(
                args.output,
                args.annotation,
                args.annotation_a,
                args.annotation_b,
                args.artifact,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
            )
        except (AnnotationManifestError, OSError) as error:
            print(f"metrics failed: {error}", file=sys.stderr)
            return 1
        counts = result["counts"]
        print(f"Metrics: {args.output} (TP {counts['tp']}, FN {counts['fn']}, TN {counts['tn']}, FP {counts['fp']})")
        return 0
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
            result = build_contract_pilot_metrics(
                args.pilot,
                args.artifact,
                args.manifest,
                args.rules_reference,
                args.zoo_manifest,
            )
            args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        except (HoldoutManifestError, OSError) as error:
            print(f"contract pilot metrics failed: {error}", file=sys.stderr)
            return 1
        counts = result["counts"]
        print(f"Contract pilot metrics: {args.output} (TP {counts['tp']}, FN {counts['fn']}, TN {counts['tn']}, FP {counts['fp']})")
        return 0
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
        try:
            validate_collection_data(data, args.manifest, args.rules_reference, args.zoo_manifest)
        except HoldoutManifestError as error:
            print(f"holdout collection produced invalid artifact: {error}", file=sys.stderr)
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
