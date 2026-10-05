"""Run a RepoPilot build over every integrity-corpus PR (v0.24 Phase D).

For each manifest entry, build a minimal Git repository from the PR's changed
source, test, and gate files at the base and head SHAs (GitHub contents API),
commit base then head, and run `repopilot review --base --head --format json`.
Only changed files exist in that repository, so graph-wide signals are out of
scope; the integrity family is file- and change-local. Results are cached
under the gitignored `.integrity-corpus/results/` with the binary's version,
and fetched files under `.integrity-corpus/files/` by SHA, so a re-run after a
detector change needs no network.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path

import integrity_corpus_github as gh
from integrity_corpus_paths import is_gate_file

SOURCE_SUFFIXES = (".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs", ".py", ".go", ".rs")
RECORDED = ("integrity.", "behavioral.test-deleted-or-emptied")


def evaluate(manifest: list[dict], cache: Path, binary: Path) -> None:
    version = subprocess.run([str(binary), "--version"], capture_output=True, text=True).stdout.strip()
    results = cache / "results"
    results.mkdir(parents=True, exist_ok=True)
    for entry in manifest:
        out = results / f"{entry['id']}.json"
        if out.exists():
            continue
        changed = changed_files(cache, entry)
        fork_point = merge_base(cache, entry)
        repo = cache / "repos" / entry["id"]
        if repo.exists():
            shutil.rmtree(repo)
        repo.mkdir(parents=True)
        base, head = build_repo(repo, entry, changed, fork_point, cache)
        review = subprocess.run(
            [str(binary), "review", str(repo), "--base", base, "--head", head, "--format", "json"],
            capture_output=True,
            text=True,
        )
        try:
            report = json.loads(review.stdout)
        except json.JSONDecodeError:
            print(f"{entry['id']}: review failed: {review.stderr.strip()[:200]}", file=sys.stderr)
            continue
        signals = [
            {key: signal.get(key) for key in ("kind", "path", "line", "detail")}
            for tier in ("definitely", "maybe", "noise")
            for signal in report.get("tiered_signals", {}).get(tier, [])
            if signal.get("kind", "").startswith(RECORDED)
        ]
        out.write_text(
            json.dumps({"binary": version, "base": fork_point, "files": len(changed), "signals": signals}, indent=1)
        )
        shutil.rmtree(repo)
        print(f"{entry['id']}: {len(signals)} integrity signal(s)", file=sys.stderr, flush=True)


def merge_base(cache: Path, entry: dict) -> str:
    """The commit the PR diff is taken against. `base_sha` is the base branch
    tip, which may include later upstream work the PR never touched."""
    path = cache / f"{entry['id']}.merge-base"
    if not path.exists():
        compare = gh.api(f"repos/{entry['repo']}/compare/{entry['base_sha']}...{entry['head_sha']}")
        path.write_text(compare["merge_base_commit"]["sha"])
    return path.read_text().strip()


def changed_files(cache: Path, entry: dict) -> list[dict]:
    path = cache / f"{entry['id']}.files.json"
    if not path.exists():
        files = gh.pr_files(entry["repo"], entry["number"])
        keep = [
            {"filename": f["filename"], "status": f["status"], "previous_filename": f.get("previous_filename")}
            for f in files
            if f["filename"].endswith(SOURCE_SUFFIXES) or is_gate_file(f["filename"])
        ]
        path.write_text(json.dumps(keep))
    return json.loads(path.read_text())


def build_repo(repo: Path, entry: dict, changed: list[dict], fork_point: str, cache: Path) -> tuple[str, str]:
    git(repo, "init", "-q")
    git(repo, "config", "user.email", "corpus@repopilot.invalid")
    git(repo, "config", "user.name", "Integrity Corpus")
    for file in changed:
        before = file.get("previous_filename") or file["filename"]
        if file["status"] != "added":
            write(repo / before, raw_file(cache, entry["repo"], before, fork_point))
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "--allow-empty", "-m", "base")
    base = git(repo, "rev-parse", "HEAD")
    for file in changed:
        before = file.get("previous_filename")
        if before and before != file["filename"]:
            (repo / before).unlink(missing_ok=True)
        target = repo / file["filename"]
        if file["status"] == "removed":
            target.unlink(missing_ok=True)
        else:
            write(target, raw_file(cache, entry["repo"], file["filename"], entry["head_sha"]))
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "--allow-empty", "-m", "head")
    return base, git(repo, "rev-parse", "HEAD")


def raw_file(cache: Path, repo: str, path: str, ref: str) -> bytes | None:
    """`gh.raw_file`, cached by commit SHA: a file at a SHA never changes."""
    stored = cache / "files" / repo / ref / path
    missing = stored.with_name(stored.name + ".missing")
    if stored.is_file():
        return stored.read_bytes()
    if missing.exists():
        return None
    content = gh.raw_file(repo, path, ref)
    stored.parent.mkdir(parents=True, exist_ok=True)
    if content is None:
        missing.touch()
    else:
        stored.write_bytes(content)
    return content


def write(path: Path, content: bytes | None) -> None:
    if content is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)


def git(repo: Path, *args: str) -> str:
    result = subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True, check=True)
    return result.stdout.strip()
