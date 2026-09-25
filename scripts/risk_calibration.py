#!/usr/bin/env python3
"""Measure risk priority distribution for a RepoPilot binary and compare two runs.

`collect` runs a binary over a corpus and writes per-rule severity x priority
counts as JSON. `compare` renders a before/after Markdown table from two
collections. Used to calibrate risk formula changes (v0.24 RP24-005) with the
same corpus before and after.

    python3 scripts/risk_calibration.py collect --repopilot /tmp/v3 --output v3.json \
        --review self=HEAD~20..HEAD
    python3 scripts/risk_calibration.py compare v3.json v4.json --output table.md

Corpus: every cloned `.zoo/<repo>` in default and strict profiles, RepoPilot's
own tree, and optional `--review label=path@base..head` change reviews (labels
keep private repositories anonymous).
"""

from __future__ import annotations

import argparse
import json
import subprocess
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PRIORITIES = ("P0", "P1", "P2", "P3")
SEVERITIES = ("CRITICAL", "HIGH", "MEDIUM", "LOW", "INFO")


def run_json(args: list[str], cwd: Path) -> dict:
    result = subprocess.run(args, cwd=cwd, capture_output=True, text=True, check=False)
    if result.returncode not in (0, 1):
        raise RuntimeError(f"{' '.join(args)} failed ({result.returncode}): {result.stderr[-400:]}")
    return json.loads(result.stdout)


def summarize(findings: list[dict], in_diff_only: bool = False) -> dict:
    by_severity: dict[str, Counter] = defaultdict(Counter)
    by_rule: dict[str, Counter] = defaultdict(Counter)
    for finding in findings:
        if in_diff_only and not finding.get("in_diff", True):
            continue
        severity = finding["severity"]
        priority = finding["risk"]["priority"]
        by_severity[severity][priority] += 1
        by_rule[f"{finding['rule_id']} [{severity}]"][priority] += 1
    return {
        "by_severity": {key: dict(value) for key, value in sorted(by_severity.items())},
        "by_rule": {key: dict(value) for key, value in sorted(by_rule.items())},
    }


def collect(binary: Path, reviews: list[str]) -> dict:
    corpus: dict[str, dict] = {}
    zoo = ROOT / ".zoo"
    targets = sorted(path for path in zoo.iterdir() if (path / ".git").exists()) if zoo.exists() else []
    targets.append(ROOT)
    for target in targets:
        name = "repopilot-self" if target == ROOT else f"zoo/{target.name}"
        for profile in ("default", "strict"):
            report = run_json(
                [str(binary), "scan", str(target), "--format", "json", "--no-progress", "--profile", profile],
                ROOT,
            )
            corpus[f"scan {profile} {name}"] = summarize(report["findings"])
    for spec in reviews:
        label, _, rest = spec.partition("=")
        path, _, revisions = rest.rpartition("@") if "@" in rest else (str(ROOT), "", rest)
        base, _, head = revisions.partition("..")
        report = run_json(
            [str(binary), "review", ".", "--base", base, "--head", head or "HEAD",
             "--format", "json", "--no-progress", "--profile", "strict"],
            Path(path),
        )
        findings = report.get("findings", [])
        corpus[f"review strict {label}"] = summarize(findings)
    version = subprocess.run([str(binary), "--version"], capture_output=True, text=True).stdout.strip()
    return {"binary_version": version, "corpus": corpus}


def totals(collection: dict) -> dict[str, Counter]:
    result: dict[str, Counter] = defaultdict(Counter)
    for entry in collection["corpus"].values():
        for severity, counts in entry["by_severity"].items():
            result[severity].update(counts)
    return result


def compare(before: dict, after: dict) -> str:
    lines = [
        "| Severity | Before P0 | Before P1 | Before P2 | Before P3 | After P0 | After P1 | After P2 | After P3 |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    before_totals, after_totals = totals(before), totals(after)
    for severity in SEVERITIES:
        b, a = before_totals.get(severity, Counter()), after_totals.get(severity, Counter())
        if not b and not a:
            continue
        lines.append(
            f"| {severity} | " + " | ".join(str(b.get(p, 0)) for p in PRIORITIES)
            + " | " + " | ".join(str(a.get(p, 0)) for p in PRIORITIES) + " |"
        )
    changed = []
    for key in sorted(set(before["corpus"]) | set(after["corpus"])):
        b_rules = before["corpus"].get(key, {}).get("by_rule", {})
        a_rules = after["corpus"].get(key, {}).get("by_rule", {})
        for rule in sorted(set(b_rules) | set(a_rules)):
            b, a = b_rules.get(rule, {}), a_rules.get(rule, {})
            if b != a:
                fmt = lambda counts: ", ".join(f"{p} {counts[p]}" for p in PRIORITIES if counts.get(p))
                changed.append(f"| {key} | `{rule}` | {fmt(b) or '-'} | {fmt(a) or '-'} |")
    lines += ["", "Rules whose priority distribution changed:", ""]
    if changed:
        lines += ["| Corpus entry | Rule [severity] | Before | After |", "|---|---|---|---|", *changed]
    else:
        lines.append("None.")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    collect_parser = sub.add_parser("collect")
    collect_parser.add_argument("--repopilot", type=Path, required=True)
    collect_parser.add_argument("--output", type=Path, required=True)
    collect_parser.add_argument("--review", action="append", default=[], metavar="LABEL=PATH@BASE..HEAD")
    compare_parser = sub.add_parser("compare")
    compare_parser.add_argument("before", type=Path)
    compare_parser.add_argument("after", type=Path)
    compare_parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.command == "collect":
        data = collect(args.repopilot.resolve(), args.review)
        args.output.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return 0
    table = compare(
        json.loads(args.before.read_text(encoding="utf-8")),
        json.loads(args.after.read_text(encoding="utf-8")),
    )
    if args.output:
        args.output.write_text(table, encoding="utf-8")
    else:
        print(table, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
