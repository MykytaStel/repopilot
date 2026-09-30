"""GitHub access for the integrity corpus, through the authenticated `gh` CLI."""

from __future__ import annotations

import json
import re
import subprocess
import time
from datetime import date, timedelta
from typing import Iterator

SEARCH_PAUSE_SECONDS = 2.2  # search API: 30 requests/minute
AGENT_MARKER = re.compile(
    r"generated with \[?claude code|co-authored-by: claude|claude\.ai/code|chatgpt\.com/codex"
    r"|\bcodex\b|devin|cursor\.com|jules|copilot",
    re.IGNORECASE,
)


def api(path: str, *fields: str, paginate: bool = False) -> object:
    cmd = ["gh", "api", "-X", "GET", path, *fields]
    if paginate:
        cmd.insert(2, "--paginate")
        cmd += ["--jq", ".[]"]
    for attempt in range(4):
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode == 0:
            if paginate:
                return [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
            return json.loads(result.stdout)
        if "rate limit" in result.stderr.lower() or "secondary" in result.stderr.lower():
            time.sleep(30 * (attempt + 1))
            continue
        raise RuntimeError(f"gh api {path} failed: {result.stderr.strip()}")
    raise RuntimeError(f"gh api {path}: rate limited")


def weekly_windows(window: tuple[str, str]) -> Iterator[str]:
    start, end = (date.fromisoformat(d) for d in window)
    while start <= end:
        stop = min(start + timedelta(days=6), end)
        yield f"{start.isoformat()}..{stop.isoformat()}"
        start = stop + timedelta(days=1)


def search_prs(qualifier: str, window: tuple[str, str]) -> Iterator[tuple[str, int]]:
    for span in weekly_windows(window):
        time.sleep(SEARCH_PAUSE_SECONDS)
        data = api("search/issues", "-f", f"q=is:pr is:merged {qualifier} created:{span}", "-f", "per_page=40")
        for item in data["items"]:
            repo = item["repository_url"].removeprefix("https://api.github.com/repos/")
            yield repo, int(item["number"])


def search_repo_human_prs(repo: str, window: tuple[str, str], agent_qualifiers: list[str]) -> Iterator[int]:
    excluded = " ".join(f"-{q}" for q in agent_qualifiers if q.startswith(("author:", "label:")))
    time.sleep(SEARCH_PAUSE_SECONDS)
    query = f"repo:{repo} is:pr is:merged created:{window[0]}..{window[1]} {excluded} -author:app/dependabot -author:app/renovate"
    data = api("search/issues", "-f", f"q={query}", "-f", "per_page=50")
    for item in data["items"]:
        yield int(item["number"])


def repo_stars(repo: str) -> int:
    data = api(f"repos/{repo}")
    if data.get("fork") or data.get("archived") or data.get("private"):
        return 0
    return int(data["stargazers_count"])


def pr_files(repo: str, number: int) -> list[dict]:
    return api(f"repos/{repo}/pulls/{number}/files", "-f", "per_page=100", paginate=True)


def pr_meta(repo: str, number: int) -> dict:
    return api(f"repos/{repo}/pulls/{number}")


def has_agent_marker(body: str) -> bool:
    return bool(AGENT_MARKER.search(body))
