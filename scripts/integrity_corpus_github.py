"""GitHub access for the integrity corpus, through the authenticated `gh` CLI."""

from __future__ import annotations

import json
import re
import subprocess
import time
from datetime import date, timedelta
from typing import Iterator

SEARCH_PAUSE_SECONDS = 6.0  # stays under GitHub's secondary search limits
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


PR_SEARCH = """
query($q: String!, $after: String) {
  search(type: ISSUE, query: $q, first: 40, after: $after) {
    pageInfo { hasNextPage endCursor }
    nodes { ... on PullRequest {
      number changedFiles baseRefOid headRefOid createdAt body
      author { login __typename }
      repository { nameWithOwner stargazerCount isFork isArchived isPrivate }
      files(first: 60) { nodes { path changeType } }
    } }
  }
}
"""


def graphql(query: str, **variables: str) -> dict:
    cmd = ["gh", "api", "graphql", "-f", f"query={query}"]
    for key, value in variables.items():
        cmd += ["-f", f"{key}={value}"]
    for attempt in range(6):
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode == 0:
            return json.loads(result.stdout)["data"]
        error = result.stderr.lower()
        transient = (
            "rate limit", "secondary", "http 50", "connection reset", "timeout", "timed out", "eof",
            "tls", "no such host", "network is unreachable", "something went wrong", "502", "503",
        )
        if any(marker in error for marker in transient):
            time.sleep(60 * (attempt + 1))
            continue
        raise RuntimeError(f"gh api graphql failed: {result.stderr.strip()}")
    raise RuntimeError("gh api graphql: rate limited")


def windows(window: tuple[str, str], days: int) -> Iterator[str]:
    start, end = (date.fromisoformat(d) for d in window)
    while start <= end:
        stop = min(start + timedelta(days=days - 1), end)
        yield f"{start.isoformat()}..{stop.isoformat()}"
        start = stop + timedelta(days=1)


def search_pr_nodes(query: str, pages: int = 1) -> Iterator[dict]:
    after = None
    for _ in range(pages):
        time.sleep(SEARCH_PAUSE_SECONDS)
        variables = {"q": query} | ({"after": after} if after else {})
        data = graphql(PR_SEARCH, **variables)["search"]
        yield from (node for node in data["nodes"] if node)
        if not data["pageInfo"]["hasNextPage"]:
            return
        after = data["pageInfo"]["endCursor"]


def search_repo_human_prs(repo: str, window: tuple[str, str], agent_qualifiers: list[str]) -> Iterator[dict]:
    excluded = " ".join(f"-{q}" for q in agent_qualifiers if q.startswith(("author:", "label:")))
    query = (
        f"repo:{repo} is:pr is:merged review:approved created:{window[0]}..{window[1]} {excluded} "
        "-author:app/dependabot -author:app/renovate"
    )
    yield from search_pr_nodes(query, pages=2)


def pr_files(repo: str, number: int) -> list[dict]:
    return api(f"repos/{repo}/pulls/{number}/files", "-f", "per_page=100", paginate=True)


def pr_meta(repo: str, number: int) -> dict:
    return api(f"repos/{repo}/pulls/{number}")


def raw_file(repo: str, path: str, ref: str) -> bytes | None:
    """A file's bytes at `ref`, or `None` when it does not exist there.

    Raises when GitHub keeps failing: a missing file would turn into a
    deleted test in the evaluated repository."""
    error = ""
    for attempt in range(4):
        result = subprocess.run(
            ["gh", "api", "-H", "Accept: application/vnd.github.raw", f"repos/{repo}/contents/{path}?ref={ref}"],
            capture_output=True,
        )
        if result.returncode == 0:
            return result.stdout
        error = result.stderr.decode(errors="replace").lower()
        if "404" in error or "not found" in error or "too large" in error:
            return None
        time.sleep(15 * (attempt + 1))
    raise RuntimeError(f"could not fetch {repo}:{path}@{ref}: {error.strip()[:200]}")


def has_agent_marker(body: str) -> bool:
    return bool(AGENT_MARKER.search(body))
