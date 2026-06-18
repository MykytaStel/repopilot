#!/usr/bin/env python3
"""RepoPilot real-repo validation "zoo".

Clones a curated set of real open-source repos (pinned to fixed commit SHAs)
into a gitignored `.zoo/` directory, scans each with RepoPilot, and snapshots
per-rule finding counts so we can track how the default profile behaves on real
diversity — not just synthetic fixtures.

Repo contents are never committed; only tests/zoo/snapshots/*.json are tracked.

Subcommands:
    list          Print the manifest as a table.
    verify-pins   Check each cloned repo HEAD matches its pinned SHA.
    clone         Shallow-fetch each repo at its pinned SHA into .zoo/<name>/.
    scan          Scan each cloned repo (default + strict) and write snapshots.
    report        Aggregate snapshots into a per-rule noise-suspect table.

Typical loop:
    python3 scripts/zoo.py clone
    python3 scripts/zoo.py scan            # writes tests/zoo/snapshots/*.json
    python3 scripts/zoo.py report          # markdown triage table

Use --bless on `scan` to (re)write committed snapshots after an intended change.
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

REPO_ROOT = Path(__file__).resolve().parent.parent
MANIFEST = REPO_ROOT / "tests" / "zoo" / "manifest.toml"
SNAPSHOT_DIR = REPO_ROOT / "tests" / "zoo" / "snapshots"
ZOO_DIR = REPO_ROOT / ".zoo"

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


# ----------------------------------------------------------------------------
# git helpers
# ----------------------------------------------------------------------------
def run(cmd: list[str], cwd: Path | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=str(cwd) if cwd else None,
        text=True,
        capture_output=True,
        check=check,
    )


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


def find_repopilot(explicit: str | None) -> list[str]:
    """Resolve the repopilot invocation: explicit > release > debug > cargo run."""
    if explicit:
        return [explicit]
    for build in ("release", "debug"):
        candidate = REPO_ROOT / "target" / build / "repopilot"
        if candidate.exists():
            return [str(candidate)]
    return ["cargo", "run", "-q", "--release", "--manifest-path", str(REPO_ROOT / "Cargo.toml"), "--"]


def scan_repo(bin_cmd: list[str], path: Path, profile: str) -> dict[str, Any]:
    proc = run(
        bin_cmd + ["scan", str(path), "--format", "json", "--profile", profile],
        check=False,
    )
    if proc.returncode not in (0, 1):  # 1 == findings present / fail-on threshold
        raise SystemExit(f"scan failed for {path} ({profile}):\n{proc.stderr.strip()}")
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"scan produced invalid JSON for {path} ({profile}): {exc}")


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


def cmd_scan(args: argparse.Namespace) -> int:
    repos = select(load_manifest(), args.only)
    bin_cmd = find_repopilot(args.repopilot)
    SNAPSHOT_DIR.mkdir(parents=True, exist_ok=True)
    drift = False
    for repo in repos:
        path = repo_path(repo)
        if not path.exists():
            print(f"  {repo['name']}: NOT CLONED (run `zoo.py clone`), skip")
            drift = True
            continue
        print(f"  scanning {repo['name']} ...")
        snap = snapshot_for(
            repo,
            scan_repo(bin_cmd, path, "default"),
            scan_repo(bin_cmd, path, "strict"),
        )
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
    return 1 if drift and not args.bless else 0


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
    p_scan.add_argument("--repopilot", help="path to repopilot binary (default: target/release|debug, else cargo run)")
    p_scan.set_defaults(func=cmd_scan)

    sub.add_parser("report", help="aggregate snapshots into a triage table").set_defaults(func=cmd_report)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
