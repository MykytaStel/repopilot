"""Render the deterministic, local-only v0.23 Phase 0 evidence audit."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import replace
from pathlib import Path
from typing import Any

from phase0_evidence_model import Phase0Paths
from phase0_evidence_tracks import differential_track, real_history_track


REPORT_SCHEMA_VERSION = 1
REPORT_PROTOCOL = "phase0-evidence-closure-v1"


def build_report(paths: Phase0Paths) -> dict[str, Any]:
    tracks = [real_history_track(paths), differential_track(paths)]
    closure_eligible = all(
        track["evidence_status"] == "valid" and track["scope"] == "independent-adjudication"
        for track in tracks
    )
    invalid = any(
        track["evidence_status"] in {"invalid", "protocol-invalid"} for track in tracks
    )
    return {
        "schema_version": REPORT_SCHEMA_VERSION,
        "protocol": REPORT_PROTOCOL,
        "status": "invalid" if invalid else ("complete" if closure_eligible else "open"),
        "closure": "closed" if closure_eligible else "open",
        "claim_boundary": "No precision, recall, or utility claim is made from unlabeled evidence.",
        "tracks": tracks,
        "limitations": [
            "This is a local evidence audit, not a product quality verdict or release approval.",
            "Hosted publication/install proof and human review remain separate gates when applicable.",
        ],
    }


def render_json(report: dict[str, Any]) -> str:
    return json.dumps(report, indent=2, sort_keys=True) + "\n"


def render_text(report: dict[str, Any]) -> str:
    lines = [f"Phase 0 evidence: {report['status']} (closure: {report['closure']})"]
    for track in report["tracks"]:
        lines.append(
            f"- {track['id']}: protocol={track['protocol_status']}; "
            f"evidence={track['evidence_status']}; scope={track['scope']}"
        )
        lines.append(f"  next: {track['next_action']}")
        if "reason" in track:
            lines.append(f"  reason: {track['reason']}")
    lines.append(report["claim_boundary"])
    return "\n".join(lines) + "\n"


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Phase 0 Evidence Audit",
        "",
        f"- Status: `{report['status']}`",
        f"- Closure: `{report['closure']}`",
        "",
        "| Track | Protocol | Evidence | Scope |",
        "| --- | --- | --- | --- |",
    ]
    for track in report["tracks"]:
        lines.append(
            f"| `{track['id']}` | `{track['protocol_status']}` | "
            f"`{track['evidence_status']}` | `{track['scope']}` |"
        )
    lines.extend(["", "## Next actions", ""])
    for track in report["tracks"]:
        lines.append(f"- `{track['id']}`: {track['next_action']}")
        if "reason" in track:
            lines.append(f"  - Reason: {track['reason']}")
    lines.extend(["", f"> {report['claim_boundary']}", ""])
    return "\n".join(lines)


def exit_code(report: dict[str, Any], *, require_complete: bool) -> int:
    if report["status"] == "invalid":
        return 1
    return int(require_complete and report["closure"] != "closed")


def _resolve(root: Path, value: Path | None) -> Path | None:
    if value is None:
        return None
    return value if value.is_absolute() else root / value


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("report",), nargs="?", default="report")
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--format", choices=("text", "markdown", "json"), default="text")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--require-complete", action="store_true")
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--differential-manifest", type=Path)
    parser.add_argument("--rules-reference", type=Path)
    parser.add_argument("--zoo-manifest", type=Path)
    for flag, destination in (
        ("real-history-artifact", "real_history_artifact"),
        ("annotation-a", "annotation_a"),
        ("annotation-b", "annotation_b"),
        ("adjudication", "adjudication"),
        ("real-history-metrics", "real_history_metrics"),
        ("differential-artifact", "differential_artifact"),
        ("differential-pilot", "differential_pilot"),
        ("differential-metrics", "differential_metrics"),
    ):
        parser.add_argument(f"--{flag}", dest=destination, type=Path)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    root = args.root.resolve()
    defaults = Phase0Paths.defaults(root)
    optional_names = (
        "real_history_artifact", "annotation_a", "annotation_b", "adjudication",
        "real_history_metrics", "differential_artifact", "differential_pilot",
        "differential_metrics",
    )
    paths = replace(
        defaults,
        manifest=_resolve(root, args.manifest) or defaults.manifest,
        differential_manifest=_resolve(root, args.differential_manifest) or defaults.differential_manifest,
        rules_reference=_resolve(root, args.rules_reference) or defaults.rules_reference,
        zoo_manifest=_resolve(root, args.zoo_manifest) or defaults.zoo_manifest,
        **{name: _resolve(root, getattr(args, name)) for name in optional_names},
    )
    report = build_report(paths)
    rendered = {"text": render_text, "markdown": render_markdown, "json": render_json}[args.format](report)
    if args.output is None:
        sys.stdout.write(rendered)
    else:
        args.output.write_text(rendered, encoding="utf-8")
    return exit_code(report, require_complete=args.require_complete)


if __name__ == "__main__":
    raise SystemExit(main())
