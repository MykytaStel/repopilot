"""Run a RepoPilot build over every integrity-corpus PR (v0.24 Phase D).

For each manifest entry, build a minimal Git repository from the PR's changed
source, test, and gate files at the base and head SHAs (GitHub contents API),
commit base then head, and run `repopilot review --base --head --format json`.
Only changed files exist in that repository, so graph-wide signals are out of
scope; the integrity family is file- and change-local. Results are cached
under the gitignored `.integrity-corpus/results/<binary-sha256>/`,
and fetched files under `.integrity-corpus/files/` by SHA, so a re-run after a
detector change needs no network.
"""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

import integrity_corpus_github as gh
from integrity_corpus_paths import is_gate_file

SOURCE_SUFFIXES = (".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs", ".py", ".go", ".rs")
RECORDED = ("integrity.", "behavioral.test-deleted-or-emptied")


def binary_sha256(binary: Path) -> str:
    """Identify the exact executable so different builds never share results."""
    return hashlib.sha256(binary.read_bytes()).hexdigest()


def load_cached_results(
    cache: Path, binary: Path | None = None, allow_legacy_cache: bool = False
) -> dict[str, dict]:
    """Load one fingerprinted run; legacy flat caches require explicit opt-in."""
    results = cache / "results"

    def read_run(run_dir: Path) -> dict[str, dict]:
        entries = {path.stem: json.loads(path.read_text()) for path in sorted(run_dir.glob("*.json"))}
        if not entries:
            raise ValueError(f"no PR results are cached in {run_dir}")
        return entries

    if binary is not None:
        digest = binary_sha256(binary)
        run_dir = results / digest
        if not run_dir.is_dir():
            raise ValueError(
                f"no cached results for {binary} (sha256 {digest}); run evaluate --bin {binary}"
            )
        return read_run(run_dir)

    runs = (
        sorted(
            path
            for path in results.iterdir()
            if path.is_dir()
            and len(path.name) == 64
            and all(char in "0123456789abcdef" for char in path.name)
        )
        if results.exists()
        else []
    )
    if len(runs) > 1:
        digests = ", ".join(path.name for path in runs)
        raise ValueError(f"multiple binary evaluations are cached ({digests}); pass report --bin <repopilot>")
    if runs:
        return read_run(runs[0])

    legacy = sorted(results.glob("*.json")) if results.exists() else []
    if not allow_legacy_cache:
        return {}
    return {path.stem: json.loads(path.read_text()) for path in legacy}


def evaluate(manifest: list[dict], cache: Path, binary: Path) -> None:
    version = subprocess.run([str(binary), "--version"], capture_output=True, text=True).stdout.strip()
    digest = binary_sha256(binary)
    results = cache / "results" / digest
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
            json.dumps(
                {
                    "binary": version,
                    "binary_sha256": digest,
                    "base": fork_point,
                    "files": len(changed),
                    "signals": signals,
                },
                indent=1,
            )
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
