"""Agent pull requests closed without merge, for comparison with merged ones.

If agents weaken tests and review filters it out, the weakening should show
up in agent PRs that were closed unmerged after discussion. Same filters as
the main sample (repos >= 100 stars, <= 60 files, a modified test file), with
`comments:>0` instead of an approving review. Ids are `icc-NNN` so the merged
corpus keeps its ids.
"""

from __future__ import annotations

import json
import sys

import integrity_corpus as corpus
import integrity_corpus_github as gh


def sample_closed(per_agent: int) -> None:
    corpus.CACHE.mkdir(exist_ok=True)
    progress = corpus.CACHE / "sample-closed-progress.jsonl"
    done: set[tuple[str, str]] = set()
    picked: list[dict] = []
    if progress.exists():
        for line in progress.read_text().splitlines():
            record = json.loads(line)
            if record["type"] == "window":
                done.add((record["agent"], record["span"]))
            else:
                picked.append(record["entry"])

    def remember(record: dict) -> None:
        with progress.open("a") as handle:
            handle.write(json.dumps(record) + "\n")

    seen = {(e["repo"], e["number"]) for e in picked}
    for agent, qualifier in corpus.AGENT_QUERIES.items():
        count = sum(1 for e in picked if e["agent"] == agent)
        for span in gh.windows(corpus.WINDOW, 2):
            if count >= per_agent:
                break
            if (agent, span) in done:
                continue
            query = f"is:pr is:closed is:unmerged comments:>0 {qualifier} created:{span}"
            try:
                nodes = list(gh.search_pr_nodes(query, pages=3))
            except RuntimeError as error:
                # A window GitHub cannot serve is skipped and recorded, not fatal.
                print(f"skipped window {agent} {span}: {str(error)[:120]}", file=sys.stderr, flush=True)
                nodes = []
            for node in nodes:
                entry = corpus.qualifies(node)
                if entry is None or (entry["repo"], entry["number"]) in seen:
                    continue
                if sum(1 for e in picked if e["repo"] == entry["repo"]) >= corpus.PER_REPO:
                    continue
                entry.update(group="agent-closed", agent=agent)
                picked.append(entry)
                seen.add((entry["repo"], entry["number"]))
                remember({"type": "entry", "entry": entry})
                count += 1
                print(f"agent-closed/{agent}: {entry['repo']}#{entry['number']}", file=sys.stderr, flush=True)
                if count >= per_agent:
                    break
            remember({"type": "window", "agent": agent, "span": span})
    picked.sort(key=lambda e: (e["repo"], e["number"]))
    for index, entry in enumerate(picked, start=1):
        entry["id"] = f"icc-{index:03d}"
    corpus.write_manifest(picked, corpus.CLOSED_MANIFEST, "agent PRs closed unmerged with comments")
    print(f"wrote {len(picked)} PRs to {corpus.CLOSED_MANIFEST.relative_to(corpus.REPO_ROOT)}")
