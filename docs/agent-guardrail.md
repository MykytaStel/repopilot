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

`snapshot` records the current HEAD to `.repopilot/snapshot.json`; when the
tree is already dirty, it also pins a baseline commit of those uncommitted
files as `refs/repopilot/snapshot`. `review --since-snapshot` then covers
everything the agent did — commits it made and edits it left uncommitted —
and leaves out work that predates the session.

To turn the review into a hard gate, add the review-signal gate. The exit
code is 1 when definitely-sensitive signals are present:

```bash
repopilot review --since-snapshot --fail-on-review definitely
```

## Pick your agent

| Agent | How RepoPilot runs | When the agent weakened a check |
|---|---|---|
| Claude Code | [plugin](#claude-code-install-the-plugin) | the stop is blocked once, with the list |
| Codex | [plugin](#codex-install-the-plugin) | the stop is blocked once, with the list |
| Cursor | [project hooks](#cursor-project-hooks) | one follow-up message with the list |
| Gemini CLI | [project hooks](#gemini-cli-project-hooks) | the reply is rejected once, with the list |
| GitHub Copilot coding agent | [setup steps, MCP, and the Action](#github-copilot-coding-agent) | the pull request review lists it; the agent is not stopped |
| Any other agent | [AGENTS.md instructions](#any-agent-agentsmd) | the agent runs the review itself, if it follows the instructions |

Every hook runs the same two scripts and the same review. Each one reports
the same signals; only how the agent hears about them differs.

## Claude Code: install the plugin

The RepoPilot plugin wires the whole loop into Claude Code. With the
`repopilot` CLI installed, run inside Claude Code:

```text
/plugin marketplace add MykytaStel/repopilot
/plugin install repopilot@repopilot
```

The plugin snapshots the repository when a session starts. When Claude tries
to stop, it reviews everything the session changed and blocks the stop once
if it finds a definitely-sensitive signal or any test-integrity signal: a
focused, skipped, or removed test, a test that lost assertions, or a new lint,
type, or coverage suppression. Claude gets the list with file and line and must
restore each check or explain why the change is intended. It also registers the
MCP server below and a `review-session` skill. Outside a Git repository, or
without the CLI, the hooks do nothing.

## Claude Code: wire the hooks by hand

The same loop without the plugin: take a snapshot when a session starts,
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

## Cursor: project hooks

The same loop for Cursor's agent uses its `sessionStart` and `stop` hooks.
From the repository root, with the `repopilot` CLI installed:

```bash
mkdir -p .cursor/hooks
curl -fsSL -o .cursor/hooks.json https://raw.githubusercontent.com/MykytaStel/repopilot/main/integrations/cursor/hooks.json
curl -fsSL -o .cursor/hooks/repopilot-snapshot.sh https://raw.githubusercontent.com/MykytaStel/repopilot/main/integrations/cursor/hooks/repopilot-snapshot.sh
curl -fsSL -o .cursor/hooks/repopilot-guard.sh https://raw.githubusercontent.com/MykytaStel/repopilot/main/integrations/cursor/hooks/repopilot-guard.sh
chmod +x .cursor/hooks/repopilot-*.sh
```

If `.cursor/hooks.json` already exists, add the two entries from
[`integrations/cursor/hooks.json`](../integrations/cursor/hooks.json) to it
instead of overwriting it.

`sessionStart` takes a snapshot. When the agent finishes a turn, `stop`
reviews everything the session changed. If it finds a definitely-sensitive
signal or any test-integrity signal, it sends the agent one follow-up message
listing each signal with its file and line, and asks the agent to restore the
check or explain why the change is intended. `loop_limit: 1` and the script's
own `loop_count` check keep it to one follow-up per stop. The hook prints `{}`
and does nothing when the turn was aborted, outside a Git repository, or when
the CLI is missing.

## Codex: install the plugin

The Claude Code plugin also ships a Codex manifest, so Codex installs it from
the same marketplace. With the `repopilot` CLI installed:

```bash
codex plugin marketplace add MykytaStel/repopilot
codex plugin add repopilot@repopilot
```

Codex asks you to trust hooks that it does not manage. Open `/hooks` in a
Codex session once and trust the two RepoPilot hooks. For `codex exec` in
automation you have already vetted, `--dangerously-bypass-hook-trust` runs them
without that step. After that, `SessionStart` takes a snapshot and `Stop` runs
the review. Codex then works the same way as Claude Code: the stop is blocked
once and the list goes back to the agent. The plugin also registers the MCP
server and the `review-session` skill.

The hooks run the `repopilot` found on `PATH`. Test-integrity signals need
RepoPilot 0.24 or newer; check with `repopilot --version`.

To register only the MCP server, run
`codex mcp add repopilot -- repopilot mcp --root .`.

## Gemini CLI: project hooks

Gemini CLI reads hooks and MCP servers from `.gemini/settings.json`. From the
repository root, with the `repopilot` CLI installed:

```bash
mkdir -p .gemini/hooks
curl -fsSL -o .gemini/settings.json https://raw.githubusercontent.com/MykytaStel/repopilot/main/integrations/gemini/settings.json
curl -fsSL -o .gemini/hooks/repopilot-snapshot.sh https://raw.githubusercontent.com/MykytaStel/repopilot/main/integrations/claude-code/repopilot/scripts/snapshot.sh
curl -fsSL -o .gemini/hooks/repopilot-guard.sh https://raw.githubusercontent.com/MykytaStel/repopilot/main/integrations/claude-code/repopilot/scripts/guard.sh
```

If `.gemini/settings.json` already exists, merge the `hooks` and `mcpServers`
entries from [`integrations/gemini/settings.json`](../integrations/gemini/settings.json)
into it instead of overwriting it.

`SessionStart` takes a snapshot. `AfterAgent` runs the review after each agent
reply. When it finds a weakened check, it rejects the reply once and sends the
list back, so the agent gets another turn to restore the check or explain the
change.

## GitHub Copilot coding agent

Copilot's coding agent works in GitHub Actions and opens a pull request. Its
hooks cannot stop the agent, so RepoPilot checks its work in two places:

1. Copy [`integrations/copilot/copilot-setup-steps.yml`](../integrations/copilot/copilot-setup-steps.yml)
   to `.github/workflows/copilot-setup-steps.yml`. The agent's environment then
   has the `repopilot` CLI and a snapshot before it starts.
2. In the repository's Copilot settings, add the MCP server from
   `repopilot init --mcp-client copilot`. The agent can then call
   `repopilot_review_change` before it finishes.
3. Review the pull request with the [GitHub Action](#gate-pull-requests-in-ci).
   This is the step that reliably catches a weakened check, because it does
   not depend on the agent.

Add the [AGENTS.md instructions](#any-agent-agentsmd) too: Copilot's coding
agent reads `AGENTS.md`.

## Any agent: AGENTS.md

Many agents read `AGENTS.md` at the repository root, including Codex, Copilot,
Cursor, and Gemini CLI. Paste
[`integrations/agents/AGENTS.md`](../integrations/agents/AGENTS.md) into yours.
It asks the agent to snapshot at the start, run `repopilot review
--since-snapshot` before it reports the task as done, and restore any check
the review lists.

These are instructions, not a gate. An agent can skip them. Where your agent
supports hooks, use the hooks as well.

## Let the agent query RepoPilot mid-task (MCP)

Generate a client config — RepoPilot never edits external client settings
itself:

```bash
repopilot init --mcp-client claude    # or: codex, copilot, cursor, gemini, generic
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

## Catch broken imports and exports, not just risky patterns

Two review-first checks look for provably broken code rather than risky
patterns, so an agent's own edits get caught before a human does:

- **`architecture.unresolved-local-import`** — an explicit local file-backed
  Rust, TypeScript/JavaScript or Python relative import, or a Go module-path
  import owned by `go.mod`, whose complete candidate set is absent.
  Ambiguous forms (Go `replace`/external modules, aliases, extensionless imports, workspace packages) stay
  bounded diagnostics, never a false broken-code claim.
- **`behavioral.removed-export-still-imported`** — a named export an agent
  deleted or renamed while a local caller still imports the old name. Fires
  only when the resolver proves the caller targets the changed module; a
  coordinated rename across both sides produces no signal.

Neither runs a compiler; both are AST-plus-resolver proofs, so they hold even
when nothing else in the diff looks risky — exactly the failure mode an agent
introduces by editing one side of an import and forgetting the other.

## Let RepoPilot run your own checks

`--verify <ID>` executes one repository-declared check (test, build,
type-check, or lint) and folds a timeout, non-zero exit, or spawn error into
the review record as evidence — never as a silent pass. Off by default;
RepoPilot never runs anything you have not explicitly allowlisted:

```toml
# repopilot.toml
[[verification.checks]]
id = "typecheck"
role = "type-check"
program = "npm"
args = ["run", "typecheck"]
timeout_seconds = 120
```

```bash
repopilot review --since-snapshot --verify typecheck
```

Static findings are never suppressed by a passing check — verification adds
evidence, it does not gate what static analysis already proved. Full contract,
including opt-in result caching: [Configuration](configuration.md#explicit-local-verification).

## What the review actually checks

Security boundaries (access control, request trust, deploy surface, supply
chain, secrets), behavioral changes (network, subprocess, filesystem, SQL,
removed error handling or auth checks), test integrity (tests newly skipped,
focused, removed, or stripped of assertions, and new lint/type/coverage
suppressions, and relaxed CI or tool gates), algorithmic shifts, taint-lite
flows
(changed request/process input reaching SQL, exec, filesystem-write, or
network sinks), broken local imports/exports (above), and blast radius
through the import graph. Signals are structural evidence with file:line
provenance — flags to verify, not verdicts. Details:
[Reports and schemas](reports.md).
