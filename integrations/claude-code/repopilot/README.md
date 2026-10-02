# RepoPilot plugin

Snapshots the repository when a coding-agent session starts, and reviews
everything the session changed before the agent finishes. If the agent
skipped, focused, removed, or weakened a test, added a lint, type, or
coverage suppression, relaxed a CI gate, or made a definitely-sensitive
change, the stop is blocked once and the agent gets the list with file and
line. It must restore each check or explain why the change is intended.

The same directory installs in Claude Code and Codex. It also registers the
local RepoPilot MCP server and a `review-session` skill.

## Requirements

The `repopilot` CLI, version 0.24 or newer, on `PATH`:
`npm install -g repopilot` or `cargo install repopilot`. Outside a Git
repository, or without the CLI, the hooks do nothing.

## Install

Claude Code:

```text
/plugin marketplace add MykytaStel/repopilot
/plugin install repopilot@repopilot
```

Codex, then trust the two hooks once in `/hooks`:

```bash
codex plugin marketplace add MykytaStel/repopilot
codex plugin add repopilot@repopilot
```

## What runs

| Hook | Script | What it does |
|---|---|---|
| `SessionStart` | `scripts/snapshot.sh` | `repopilot snapshot` at the repository root |
| `Stop` | `scripts/guard.sh` | `repopilot review --since-snapshot`; exit 2 with the flagged signals, once per stop |

Everything runs locally. RepoPilot sends no source code anywhere and calls
no language model.

More: [Guard your agent runs](https://github.com/MykytaStel/repopilot/blob/main/docs/agent-guardrail.md).
