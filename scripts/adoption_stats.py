#!/usr/bin/env python3
"""Who uses RepoPilot: one Markdown page from public and owner-only counters.

GitHub traffic (views, clones, referrers) needs `gh` logged in as a repository
owner; everything else is public. Raw download counts mostly measure mirrors
and bots, so each section says how to read its number.

    python3 scripts/adoption_stats.py                      # print the page
    python3 scripts/adoption_stats.py --history FILE.jsonl # also append the numbers
"""

from __future__ import annotations

import argparse
import json
import subprocess
import urllib.request
from datetime import date

REPO = "MykytaStel/repopilot"
NPM_PLATFORMS = ("darwin-arm64", "darwin-x64", "linux-x64-gnu", "linux-arm64-gnu", "win32-x64-msvc")
USER_AGENT = f"repopilot-adoption-stats (https://github.com/{REPO})"
MCP_NAME = "io.github.MykytaStel/repopilot"


def fetch(url: str) -> dict:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "application/json"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def gh(path: str):
    run = subprocess.run(["gh", "api", path], capture_output=True, text=True)
    return json.loads(run.stdout) if run.returncode == 0 else None


def npm_week(package: str) -> int:
    return fetch(f"https://api.npmjs.org/downloads/point/last-week/{package}").get("downloads", 0)


def collect() -> dict:
    repo = gh(f"repos/{REPO}") or {}
    views = gh(f"repos/{REPO}/traffic/views") or {}
    clones = gh(f"repos/{REPO}/traffic/clones") or {}
    platforms = {name: npm_week(f"@repopilot/{name}") for name in NPM_PLATFORMS}
    crate = fetch("https://crates.io/api/v1/crates/repopilot")
    releases = gh(f"repos/{REPO}/releases?per_page=5") or []
    search = subprocess.run(
        ["gh", "search", "code", REPO, "--limit", "100", "--json", "repository"],
        capture_output=True, text=True,
    )  # fmt: skip
    mentions = sorted(
        {hit["repository"]["nameWithOwner"] for hit in json.loads(search.stdout or "[]")} - {REPO}
    )
    registry = fetch(f"https://registry.modelcontextprotocol.io/v0/servers?search={MCP_NAME.split('/')[1]}")
    listed = any(entry.get("server", entry).get("name") == MCP_NAME for entry in registry.get("servers", []))
    return {
        "date": date.today().isoformat(),
        "stars": repo.get("stargazers_count"),
        "forks": repo.get("forks_count"),
        "views_14d": views.get("count"),
        "visitors_14d": views.get("uniques"),
        "clones_14d": clones.get("count"),
        "cloners_14d": clones.get("uniques"),
        "referrers": [(r["referrer"], r["uniques"]) for r in gh(f"repos/{REPO}/traffic/popular/referrers") or []],
        "npm_week": npm_week("repopilot"),
        "npm_platforms_week": platforms,
        "crates_total": crate["crate"]["downloads"],
        "crates_90d": crate["crate"].get("recent_downloads"),
        "crates_versions": [(v["num"], v["downloads"]) for v in crate["versions"][:3]],
        "releases": [(r["tag_name"], sum(a["download_count"] for a in r["assets"])) for r in releases],
        "mentions": mentions,
        "mcp_registry": listed,
    }


def render(stats: dict) -> str:
    platforms = stats["npm_platforms_week"]
    floor = min(platforms.values()) if platforms else 0
    above_floor = sum(count - floor for count in platforms.values())
    referrers = ", ".join(f"{name} ({uniques})" for name, uniques in stats["referrers"]) or "none"
    lines = [
        f"# RepoPilot adoption, {stats['date']}",
        "",
        "| Counter | Value | How to read it |",
        "|---|---|---|",
        f"| GitHub stars / forks | {stats['stars']} / {stats['forks']} | |",
        f"| Repository visitors, 14 days | {stats['visitors_14d']} ({stats['views_14d']} views) | real people, including you |",
        f"| Referring sites, 14 days | {referrers} | where visitors came from; check after every post |",
        f"| Clones, 14 days | {stats['cloners_14d']} unique ({stats['clones_14d']} total) | mostly CI and bots; plugin installs hide in here |",
        f"| npm `repopilot`, last week | {stats['npm_week']} | mirrors fetch every new version |",
        f"| npm platform packages, last week | {', '.join(f'{k} {v}' for k, v in platforms.items())} "
        f"| a real install fetches one platform; ~{above_floor} above the common floor of {floor} |",
        f"| crates.io total / 90 days | {stats['crates_total']} / {stats['crates_90d']} | ~400 per version is the mirror baseline |",
        f"| crates.io latest versions | {', '.join(f'{v} {n}' for v, n in stats['crates_versions'])} | |",
        f"| Release binary downloads | {', '.join(f'{t} {n}' for t, n in stats['releases'])} | the Action and installers fetch these |",
        f"| Other repositories naming `{REPO}` | {len(stats['mentions'])}: {', '.join(stats['mentions'][:8]) or '—'} | workflows here are real Action users |",
        f"| MCP Registry `{MCP_NAME}` | {'listed' if stats['mcp_registry'] else 'not listed'} | |",
    ]
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--history", help="append this run's numbers as one JSON line")
    args = parser.parse_args()
    stats = collect()
    print(render(stats))
    if args.history:
        with open(args.history, "a") as handle:
            handle.write(json.dumps(stats) + "\n")


if __name__ == "__main__":
    main()
