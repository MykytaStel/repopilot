# Guard Your Agent Runs

Coding agents change more code per hour than anyone reviews carefully. This
page wires RepoPilot in as the deterministic layer around a run: snapshot
before, review after, gate on the high-confidence tier. Everything here is
local and offline; nothing needs an API key.

## The core loop

```bash
repopilot snapshot            # before the agent starts
# ... agent session ...
repopilot review --since-snapshot
```

`snapshot` records the current HEAD (and whether the tree was already dirty)
to `.repopilot/snapshot.json`. `review --since-snapshot` then covers
everything the agent did — commits it made and edits it left uncommitted —
not just the current working tree.

To turn the review into a hard gate, add the review-signal gate. The exit
code is 1 when definitely-sensitive signals are present:

```bash
repopilot review --since-snapshot --fail-on-review definitely
```

## Claude Code: review every session automatically

Two hooks make the loop invisible: take a snapshot when a session starts,
review the session when the agent tries to stop. If the review gate fails,
the agent sees the signals and must address them (or explain them) before
finishing.

`.claude/hooks/repopilot-guard.sh`:

```bash
#!/usr/bin/env bash
set -uo pipefail

# Stop hook: review everything the agent did since the session snapshot.
# Requires jq. Exit 2 blocks the stop and feeds stderr back to the agent.
input=$(cat)
if [ "$(jq -r '.stop_hook_active // false' <<<"$input")" = "true" ]; then
  exit 0  # already re-prompted once; don't loop
fi

out=$(repopilot review --since-snapshot --fail-on-review definitely 2>&1)
if [ $? -ne 0 ]; then
  echo "RepoPilot flagged definitely-sensitive changes in this session:" >&2
  echo "$out" >&2
  exit 2
fi
```

`.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          { "type": "command", "command": "repopilot snapshot >/dev/null 2>&1 || true" }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          { "type": "command", "command": "bash .claude/hooks/repopilot-guard.sh" }
        ]
      }
    ]
  }
}
```

Notes:

- The gate only fires on **definitely**-sensitive signals (removed auth
  checks, taint flows, boundary changes) — advisory "maybe" signals never
  block a session.
- The `stop_hook_active` check means the agent is re-prompted at most once
  per stop; it can resolve the signals or explicitly justify them.
- On repositories with existing debt this stays quiet: review signals are
  computed from the session's diff, not the whole repository.

## Let the agent query RepoPilot mid-task (MCP)

Generate a client config — RepoPilot never edits external client settings
itself:

```bash
repopilot init --mcp-client claude    # or: cursor, generic
```

The MCP server (`repopilot mcp --root .`) is synchronous, root-confined, and
makes no network calls. The tools agents use most:

| Tool | What it answers |
|---|---|
| `repopilot_review_change` | "What did my change touch?" — signals, findings, blast radius, gate result |
| `repopilot_context` | Budgeted Markdown context about the repository |
| `repopilot_explain_review_signal` | Provenance and verification steps for one signal |

Full contract: [MCP server](mcp.md).

## Gate pull requests in CI

Generate a review-first GitHub Actions workflow:

```bash
repopilot init --github-action
```

Or call the gate directly in any CI:

```bash
repopilot review . --base "origin/${BASE_BRANCH:-main}" --fail-on-review definitely
```

The finding gate (`--fail-on`) evaluates only in-diff findings, so
pre-existing issues never block an unrelated PR. On repositories with known
debt, `repopilot baseline create .` pins the current state so only new
findings count. See
[GitHub pull request integration](integrations/github-code-scanning.md).

## What the review actually checks

Security boundaries (access control, request trust, deploy surface, supply
chain, secrets), behavioral changes (network, subprocess, filesystem, SQL,
removed error handling or auth checks), algorithmic shifts, taint-lite flows
(changed request/process input reaching SQL, exec, filesystem-write, or
network sinks), and blast radius through the import graph. Signals are
structural evidence with file:line provenance — flags to verify, not
verdicts. Details: [Reports and schemas](reports.md).
