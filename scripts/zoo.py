#!/usr/bin/env python3
"""RepoPilot real-repo validation "zoo".

Clones pinned real repositories into the gitignored `.zoo/`, scans them with the
current workspace RepoPilot by default, and compares committed snapshots. Repo
contents are never committed; only tests/zoo/snapshots/*.json are tracked.

See tests/zoo/README.md for the full workflow.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tomllib
from collections import Counter
from pathlib import Path
from typing import Any

import zoo_expectations as ze
import zoo_scorecard as zs
import zoo_triage as zt
from zoo_scanner import (
    ScannerPreparationError,
    ScannerProvenanceError,
    prepare_scanner,
    print_scanner_provenance,
    run,
    scan_repo,
)

REPO_ROOT = Path(__file__).resolve().parent.parent
MANIFEST = REPO_ROOT / "tests" / "zoo" / "manifest.toml"
SNAPSHOT_DIR = REPO_ROOT / "tests" / "zoo" / "snapshots"
EXPECTATION_DIR = REPO_ROOT / "tests" / "zoo" / "expectations"
ZOO_DIR = REPO_ROOT / ".zoo"
RULES_REFERENCE = REPO_ROOT / "docs" / "rules-reference.md"
RULE_SCORECARD = REPO_ROOT / "docs" / "engineering" / "rule-scorecard.md"

# Default-visible volume above this on a single repo flags a rule as a
# false-positive suspect in the `report` table (real code rarely has many
# genuine, default-visible findings of one rule).
NOISE_SUSPECT_PER_REPO = 5


# ----------------------------------------------------------------------------
# Manifest
# ----------------------------------------------------------------------------
def load_manifest() -> list[dict[str, Any]]:
    if not MANIFEST.exists():
        raise SystemExit(f"zoo manifest not found: {MANIFEST}")
    data = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    repos = data.get("repo")
    if not isinstance(repos, list) or not repos:
        raise SystemExit("zoo manifest has no [[repo]] entries")
    for repo in repos:
        for key in ("name", "url", "sha", "language"):
            if not repo.get(key):
                raise SystemExit(f"manifest entry missing '{key}': {repo}")
    return repos


def repo_path(repo: dict[str, Any]) -> Path:
    return ZOO_DIR / repo["name"]


def head_sha(path: Path) -> str | None:
    try:
        return run(["git", "rev-parse", "HEAD"], cwd=path).stdout.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None


# ----------------------------------------------------------------------------
# Subcommands
# ----------------------------------------------------------------------------
def cmd_list(_args: argparse.Namespace) -> int:
    repos = load_manifest()
    print(f"{'name':18} {'language':12} {'framework':14} sha")
    for repo in repos:
        print(
            f"{repo['name']:18} {repo['language']:12} "
            f"{(repo.get('framework') or '-'):14} {repo['sha'][:12]}"
        )
    print(f"\n{len(repos)} repos. Clone dir: {ZOO_DIR}")
    return 0


def cmd_verify_pins(_args: argparse.Namespace) -> int:
    repos = load_manifest()
    mismatches: list[str] = []
    missing: list[str] = []
    for repo in repos:
        path = repo_path(repo)
        if not path.exists():
            missing.append(repo["name"])
            continue
        actual = head_sha(path)
        if actual != repo["sha"]:
            mismatches.append(f"{repo['name']}: pinned {repo['sha'][:12]}, got {(actual or 'n/a')[:12]}")
    if missing:
        print(f"not cloned (run `zoo.py clone`): {', '.join(missing)}")
    for line in mismatches:
        print(f"PIN MISMATCH {line}")
    if mismatches:
        return 1
    if not missing:
        print(f"all {len(repos)} pins match")
    return 0


def clone_one(repo: dict[str, Any], force: bool) -> bool:
    """Fetch a single repo at its pinned SHA. Returns True on success."""
    path = repo_path(repo)
    if path.exists():
        if not force and head_sha(path) == repo["sha"]:
            print(f"  {repo['name']}: already at pin, skip")
            return True
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)
    try:
        run(["git", "init", "-q"], cwd=path)
        run(["git", "remote", "add", "origin", repo["url"]], cwd=path)
        # Fetch only the pinned commit at depth 1 (GitHub allows fetch-by-SHA).
        run(["git", "fetch", "-q", "--depth", "1", "origin", repo["sha"]], cwd=path)
        run(["git", "checkout", "-q", repo["sha"]], cwd=path)
    except subprocess.CalledProcessError as exc:
        print(f"  {repo['name']}: FETCH FAILED\n{exc.stderr.strip()}")
        return False
    print(f"  {repo['name']}: cloned @ {repo['sha'][:12]}")
    return True


def cmd_clone(args: argparse.Namespace) -> int:
    repos = select(load_manifest(), args.only)
    print(f"Cloning {len(repos)} repos into {ZOO_DIR} ...")
    failures = [r["name"] for r in repos if not clone_one(r, args.force)]
    if failures:
        print(f"FAILED: {', '.join(failures)}")
        return 1
    return 0


def visible_findings(report: dict[str, Any]) -> list[dict[str, Any]]:
    findings = report.get("findings")
    return [f for f in findings if isinstance(f, dict)] if isinstance(findings, list) else []


def rule_of(finding: dict[str, Any]) -> str:
    return finding.get("rule_id") or "unknown"


def priority_of(finding: dict[str, Any]) -> str:
    risk = finding.get("risk")
    if isinstance(risk, dict) and isinstance(risk.get("priority"), str):
        return risk["priority"]
    return "unknown"


def snapshot_for(repo: dict[str, Any], default_report: dict[str, Any], strict_report: dict[str, Any]) -> dict[str, Any]:
    default = visible_findings(default_report)
    strict = visible_findings(strict_report)
    return {
        "repo": repo["name"],
        "sha": repo["sha"],
        "language": repo["language"],
        "framework": repo.get("framework") or "",
        # Default profile is what users see — track it precisely (fingerprints).
        "default": {
            "visible_total": len(default),
            "by_rule": dict(sorted(Counter(rule_of(f) for f in default).items())),
            "by_priority": dict(sorted(Counter(priority_of(f) for f in default).items())),
            "fingerprints": sorted(f.get("id", "") for f in default),
        },
        # Strict can be huge on real repos — counts only, no fingerprints.
        "strict": {
            "visible_total": len(strict),
            "by_rule": dict(sorted(Counter(rule_of(f) for f in strict).items())),
        },
    }


def expectation_path(repo_name: str) -> Path:
    return EXPECTATION_DIR / f"{repo_name}.toml"


def cmd_scan(args: argparse.Namespace) -> int:
    repos = select(load_manifest(), args.only)
    try:
        scanner = prepare_scanner(
            REPO_ROOT,
            args.repopilot,
            args.allow_version_mismatch,
        )
    except ScannerPreparationError as exc:
        print(f"SCANNER PREPARATION FAILED\n{exc}", file=sys.stderr)
        return 2
    except ScannerProvenanceError as exc:
        print(f"SCANNER PROVENANCE FAILED\n{exc}", file=sys.stderr)
        return 3

    print_scanner_provenance(scanner)
    SNAPSHOT_DIR.mkdir(parents=True, exist_ok=True)
    drift = False
    live: dict[str, tuple[list[Any], list[Any]]] = {}
    for repo in repos:
        path = repo_path(repo)
        if not path.exists():
            print(f"  {repo['name']}: NOT CLONED (run `zoo.py clone`), skip")
            drift = True
            continue
        print(f"  scanning {repo['name']} ...")
        default_report = scan_repo(scanner, path, "default")
        strict_report = scan_repo(scanner, path, "strict")
        live[repo["name"]] = (
            ze.parse_live_findings(default_report, path, "default"),
            ze.parse_live_findings(strict_report, path, "strict"),
        )
        snap = snapshot_for(repo, default_report, strict_report)
        out = SNAPSHOT_DIR / f"{repo['name']}.json"
        rendered = json.dumps(snap, indent=2, sort_keys=True) + "\n"
        if args.bless or not out.exists():
            out.write_text(rendered, encoding="utf-8")
            print(f"    wrote {out.relative_to(REPO_ROOT)} (default-visible: {snap['default']['visible_total']})")
        else:
            previous = out.read_text(encoding="utf-8")
            if previous != rendered:
                drift = True
                print(f"    DRIFT vs committed snapshot {out.relative_to(REPO_ROOT)} (re-run with --bless to accept)")
            else:
                print(f"    ok (default-visible: {snap['default']['visible_total']})")

    exp_diags = validate_expectations(repos, live)
    print_expectation_diagnostics(exp_diags)
    expectation_failed = any(ze.is_failure(d.kind) for d in exp_diags)
    snapshot_failed = drift and not args.bless
    return 1 if snapshot_failed or expectation_failed else 0


def validate_expectations(
    selected: list[dict[str, Any]],
    live: dict[str, tuple[list[Any], list[Any]]],
) -> list[ze.Diagnostic]:
    """Parse + match expectation files for the selected repos against live findings."""
    if not EXPECTATION_DIR.exists():
        print("  no expectations directory; skipping expectation validation")
        return []
    manifest_names = [r["name"] for r in load_manifest()]
    present_stems = [p.stem for p in EXPECTATION_DIR.glob("*.toml")]
    required = [r["name"] for r in selected]
    diags = list(ze.coverage_diagnostics(required, manifest_names, present_stems))
    present = set(present_stems)
    for repo in selected:
        name = repo["name"]
        if name not in present:
            continue  # MISSING EXPECTATION FILE already reported by coverage_diagnostics
        expectation, pdiags = ze.parse_expectation_file(expectation_path(name), name)
        diags += pdiags
        if expectation is None or name not in live:
            continue
        default_lives, strict_lives = live[name]
        mdiags, _review = ze.match_repo(name, expectation, default_lives, strict_lives)
        diags += mdiags
    return diags


def print_expectation_diagnostics(diags: list[ze.Diagnostic]) -> None:
    if not diags:
        print("\nexpectations: every default-visible finding labeled; strict anchors intact")
        return
    by_kind: dict[str, list[ze.Diagnostic]] = {}
    for diag in diags:
        by_kind.setdefault(diag.kind, []).append(diag)
    print("\n== expectation review ==")
    for kind in ze.DIAGNOSTIC_ORDER:
        items = by_kind.get(kind)
        if not items:
            continue
        tag = "FAIL" if ze.is_failure(kind) else "info"
        print(f"\n[{tag}] {kind} ({len(items)})")
        for diag in sorted(items, key=lambda d: (d.repo, d.message)):
            print(f"  - {diag.repo}: {diag.message}")


def cmd_report(_args: argparse.Namespace) -> int:
    snaps = sorted(SNAPSHOT_DIR.glob("*.json"))
    if not snaps:
        raise SystemExit("no snapshots found; run `zoo.py scan` first")
    per_rule: Counter[str] = Counter()
    per_rule_repos: dict[str, set[str]] = {}
    suspects: list[str] = []
    print("# Zoo triage — default-visible findings per rule\n")
    print(f"{'rule_id':46} {'total':>6} {'repos':>6}")
    for path in snaps:
        snap = json.loads(path.read_text(encoding="utf-8"))
        by_rule = snap["default"]["by_rule"]
        for rule, count in by_rule.items():
            per_rule[rule] += count
            per_rule_repos.setdefault(rule, set()).add(snap["repo"])
            if count >= NOISE_SUSPECT_PER_REPO:
                suspects.append(f"{rule} — {count} in {snap['repo']}")
    for rule, total in per_rule.most_common():
        print(f"{rule:46} {total:>6} {len(per_rule_repos[rule]):>6}")
    print(f"\nTotal default-visible across {len(snaps)} repos: {sum(per_rule.values())}")
    if suspects:
        print(f"\n## Noise suspects (>= {NOISE_SUSPECT_PER_REPO} default-visible on one repo)")
        for line in suspects:
            print(f"  - {line}")
    report_reviewed_quality(snaps)
    return 0


def report_reviewed_quality(snaps: list[Path]) -> None:
    """Reviewed signal-quality summary derived from committed snapshots + labels.

    This reads only committed data (no clones): default-visible totals come from
    snapshots, dispositions from expectation files. Live-only checks (stale /
    missing anchor) are validated by `scan`; here a labeled vs snapshot-total
    mismatch is surfaced as a coverage gap to re-run `scan`.
    """
    totals = ze.RepoReview(repo="(all)")
    by_rule: Counter[str] = Counter()
    per_repo: list[tuple[str, ze.RepoReview]] = []
    coverage_gaps: list[str] = []
    fp_lines: list[str] = []
    for path in snaps:
        snap = json.loads(path.read_text(encoding="utf-8"))
        repo = snap["repo"]
        review = ze.RepoReview(repo=repo, default_total=snap["default"]["visible_total"])
        expectation, _ = ze.parse_expectation_file(expectation_path(repo), repo)
        if expectation is not None:
            for finding in expectation.findings:
                if finding.profile == "strict":
                    review.strict_anchors += 1
                    continue
                review.labeled += 1
                by_rule[finding.rule_id] += 1
                if finding.disposition == "actionable":
                    review.actionable += 1
                elif finding.disposition == "valid-but-accepted":
                    review.valid_but_accepted += 1
                elif finding.disposition == "false-positive":
                    review.false_positive += 1
                    fp_lines.append(f"{repo}: {finding.rule_id} @ {finding.path} — {finding.reason}")
        review.unlabeled = max(0, review.default_total - review.labeled)
        if review.labeled != review.default_total:
            coverage_gaps.append(
                f"{repo}: {review.labeled} labeled vs {review.default_total} default-visible (run `scan`)"
            )
        per_repo.append((repo, review))
        for attr in ("default_total", "labeled", "unlabeled", "actionable", "valid_but_accepted", "false_positive", "strict_anchors"):
            setattr(totals, attr, getattr(totals, attr) + getattr(review, attr))

    reviewed = totals.actionable + totals.valid_but_accepted + totals.false_positive
    print("\n# Reviewed signal quality (snapshots x labels)\n")
    print(f"default-visible findings : {totals.default_total}")
    print(f"labeled default findings : {totals.labeled}")
    print(f"unlabeled default        : {totals.unlabeled}")
    print(f"  actionable             : {totals.actionable}")
    print(f"  valid-but-accepted     : {totals.valid_but_accepted}")
    print(f"  false-positive         : {totals.false_positive}")
    print(f"strict recall anchors    : {totals.strict_anchors}")
    if reviewed:
        proxy = (totals.actionable + totals.valid_but_accepted) / reviewed
        rate = totals.actionable / reviewed
        print(f"reviewed precision proxy : {proxy:.2f}  ((actionable+accepted)/reviewed; not measured precision)")
        print(f"actionability rate       : {rate:.2f}  (actionable/reviewed)")

    print("\n## Labeled default by rule")
    for rule, count in sorted(by_rule.items()):
        print(f"  {rule:46} {count:>4}")
    print("\n## By repository (default_total / labeled / actionable / accepted / false-positive / strict-anchors)")
    for repo, review in per_repo:
        print(
            f"  {repo:18} {review.default_total:>3} / {review.labeled:>3} / {review.actionable:>3} / "
            f"{review.valid_but_accepted:>3} / {review.false_positive:>3} / {review.strict_anchors:>3}"
        )
    if coverage_gaps:
        print("\n## Coverage gaps (labeled != snapshot default-visible)")
        for line in coverage_gaps:
            print(f"  - {line}")
    if fp_lines:
        print(f"\n## KNOWN FALSE POSITIVES ({len(fp_lines)}) — calibration debt, reported not suppressed")
        for line in fp_lines:
            print(f"  - {line}")


def cmd_scorecard(args: argparse.Namespace) -> int:
    """Regenerate the per-rule scorecard from committed expectations (no clones)."""
    rendered = zs.generate_scorecard(load_manifest(), EXPECTATION_DIR, RULES_REFERENCE)
    if args.write:
        RULE_SCORECARD.parent.mkdir(parents=True, exist_ok=True)
        RULE_SCORECARD.write_text(rendered, encoding="utf-8")
        print(f"wrote {RULE_SCORECARD.relative_to(REPO_ROOT)}")
        return 0
    if not RULE_SCORECARD.exists():
        print(f"MISSING {RULE_SCORECARD.relative_to(REPO_ROOT)} (run `zoo.py scorecard --write`)")
        return 1
    committed = RULE_SCORECARD.read_text(encoding="utf-8")
    if committed != rendered:
        print(f"STALE {RULE_SCORECARD.relative_to(REPO_ROOT)} (re-run with --write to refresh)")
        return 1
    print(f"ok ({RULE_SCORECARD.relative_to(REPO_ROOT)} matches committed expectations)")
    return 0


def cmd_triage(args: argparse.Namespace) -> int:
    repos = select(load_manifest(), args.only)
    try:
        scanner = prepare_scanner(REPO_ROOT, args.repopilot, args.allow_version_mismatch)
    except ScannerPreparationError as exc:
        print(f"SCANNER PREPARATION FAILED\n{exc}", file=sys.stderr)
        return 2
    except ScannerProvenanceError as exc:
        print(f"SCANNER PROVENANCE FAILED\n{exc}", file=sys.stderr)
        return 3

    print_scanner_provenance(scanner)
    any_unlabeled = False
    for repo in repos:
        path = repo_path(repo)
        if not path.exists():
            print(f"  {repo['name']}: NOT CLONED (run `zoo.py clone`), skip")
            continue
        default_lives = ze.parse_live_findings(scan_repo(scanner, path, "default"), path, "default")
        expectation, _ = ze.parse_expectation_file(expectation_path(repo["name"]), repo["name"])
        unlabeled = ze.unlabeled_default(expectation, default_lives)
        if not unlabeled:
            continue
        any_unlabeled = True
        print(f"\n===== {repo['name']}: {len(unlabeled)} unlabeled default finding(s) =====\n")
        for live in unlabeled:
            print(zt.render_triage_entry(repo["name"], live))
        if args.write:
            out = expectation_path(repo["name"])
            if out.exists():
                print(f"  (expectation file {out.name} exists; paste the skeleton(s) above and label them)")
            else:
                EXPECTATION_DIR.mkdir(parents=True, exist_ok=True)
                out.write_text(zt.skeleton_file_text(repo["name"], unlabeled), encoding="utf-8")
                print(f"  wrote REVIEW skeleton {out.relative_to(REPO_ROOT)} (every disposition is REVIEW_REQUIRED)")
    if not any_unlabeled:
        print("\nno unlabeled default findings — every default-visible finding is labeled")
    return 0


def select(repos: list[dict[str, Any]], only: str | None) -> list[dict[str, Any]]:
    if not only:
        return repos
    wanted = {name.strip() for name in only.split(",") if name.strip()}
    chosen = [r for r in repos if r["name"] in wanted]
    missing = wanted - {r["name"] for r in chosen}
    if missing:
        raise SystemExit(f"unknown repo(s): {', '.join(sorted(missing))}")
    return chosen


def main() -> int:
    parser = argparse.ArgumentParser(description="RepoPilot real-repo validation zoo.")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("list", help="print the manifest").set_defaults(func=cmd_list)
    sub.add_parser("verify-pins", help="check clones match pinned SHAs").set_defaults(func=cmd_verify_pins)

    p_clone = sub.add_parser("clone", help="clone repos at pinned SHAs")
    p_clone.add_argument("--only", help="comma-separated repo names")
    p_clone.add_argument("--force", action="store_true", help="re-clone even if pin matches")
    p_clone.set_defaults(func=cmd_clone)

    p_scan = sub.add_parser("scan", help="scan repos and write/check snapshots")
    p_scan.add_argument("--only", help="comma-separated repo names")
    p_scan.add_argument("--bless", action="store_true", help="(re)write committed snapshots")
    p_scan.add_argument(
        "--repopilot",
        help="explicit path to an external repopilot binary (default: build current workspace)",
    )
    p_scan.add_argument(
        "--allow-version-mismatch",
        action="store_true",
        help="allow --repopilot to use a version that differs from this workspace",
    )
    p_scan.set_defaults(func=cmd_scan)

    p_triage = sub.add_parser("triage", help="print unlabeled default findings + TOML skeletons")
    p_triage.add_argument("--only", help="comma-separated repo names")
    p_triage.add_argument(
        "--write",
        action="store_true",
        help="write a REVIEW_REQUIRED skeleton file for repos that have none (never guesses a disposition)",
    )
    p_triage.add_argument("--repopilot", help="explicit path to an external repopilot binary")
    p_triage.add_argument(
        "--allow-version-mismatch",
        action="store_true",
        help="allow --repopilot to use a version that differs from this workspace",
    )
    p_triage.set_defaults(func=cmd_triage)

    sub.add_parser("report", help="aggregate snapshots into a triage table").set_defaults(func=cmd_report)

    p_scorecard = sub.add_parser(
        "scorecard", help="check or write the per-rule scorecard (no clones needed)"
    )
    p_scorecard.add_argument("--write", action="store_true", help="regenerate the committed scorecard doc")
    p_scorecard.set_defaults(func=cmd_scorecard)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
