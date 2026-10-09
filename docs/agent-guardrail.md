# Guard Your Agent Runs

Coding agents change more code per hour than anyone reviews carefully. This
page wires RepoPilot in as the deterministic layer around a run: snapshot
before, review after, gate on the high-confidence tier. RepoPilot's analysis is
local and offline and needs no API key; installation uses your package channel.

## First five minutes

1. Install the CLI separately from the agent plugin:
   `npm install -g repopilot` or `cargo install repopilot --locked`.
2. Run `command -v repopilot` and `repopilot --version` in the terminal that
   launches your agent. Session integrity review requires **0.24 or newer**.
   The current stable release is 0.25.0. If an older executable resolves,
   update that installation and check its version before starting a session.
3. Install the plugin or extension for your agent below. Review any hook trust
   prompt, then start a new session in a Git repository.
4. Make one small change. Ask the agent to run
   `repopilot review --since-snapshot --detail full` and explain `Decision`,
   `Evidence scope`, and the named signals. You can run the same command yourself.

The CLI is the analyzer, hooks start it automatically, and MCP lets the agent
request analysis on demand. Installing only MCP gives tools to the agent;
the session-start and stop hooks provide the automatic loop. RepoPilot performs
the analysis locally. Hook feedback and MCP results go to your chosen agent,
whose own model/data handling follows that client's settings.

The shipped hook scripts need Git, a POSIX `sh`, and standard shell utilities.
They are tested on Unix. On Windows the native CLI and MCP work without these
hooks; shell hooks need a compatible environment such as Git Bash or WSL and
are not covered by the native Windows smoke tests.

## Know whether review ran

Healthy hooks stay quiet unless they find a sensitive change. A missing or old
CLI, failed snapshot, missing snapshot, or failed review produces a diagnostic
on the hook's stderr, where your agent's hook logs record it. These nonblocking
diagnostics may not appear in the visible conversation. An unavailable review
lets the agent continue and is not a clean-review result. An unavailable new
session start invalidates the previous session's snapshot metadata when the
state directory and metadata are writable. If permissions prevent invalidation,
the hooks report it and refuse review while the directory or metadata is
unwritable. Restore
write permission and start a new session before reviewing; repairing permission
alone does not establish a baseline for the failed session.

Use `repopilot review --since-snapshot --detail full` for the full human-readable
report. `REVIEW` means attention is needed; read its reasons and coverage.
`VERIFIED` requires sufficient configured verification evidence, not just a
quiet hook. The stop guard sends at most one follow-up; after that the agent may
finish, so inspect its explanation and the report before merging.

One working tree has one `.repopilot/snapshot.json`. Run one agent session at a
time in that tree, and do not replace its snapshot mid-session. Parallel agents
need separate Git worktrees so their baselines do not overwrite each other.

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
| Gemini CLI | [extension](#gemini-cli-install-the-extension) or project hooks | the reply is rejected once, with the list |
| Google Antigravity | [MCP and manual review](#google-antigravity) | automatic stop unverified |
| GitHub Copilot coding agent | [setup steps, MCP, and the Action](#github-copilot-coding-agent) | the pull request review lists it; the agent is not stopped |
| Any other agent | [AGENTS.md instructions](#any-agent-agentsmd) | the agent runs the review itself, if it follows the instructions |

The integrations use the same local review and preserve its structured MCP
signals. Automatic stop hooks block only the configured integrity and security
signals. Other candidates, including quiet fallback, stay non-gating and are
available to any configured MCP client in `tiered_signals`; inspect them as
evidence rather than a verdict.

## Claude Code: install the plugin

The RepoPilot plugin wires the whole loop into Claude Code. With the
`repopilot` CLI installed, run inside Claude Code:

```text
/plugin marketplace add MykytaStel/repopilot
/plugin install repopilot@repopilot
```

The plugin snapshots the repository when a session starts. When Claude tries
to stop, it reviews everything the session changed and blocks the stop when
the session weakened a check or a safeguard: a focused, skipped, or removed
test, a test that lost assertions, a new lint, type, coverage, or RepoPilot
suppression, a relaxed CI gate, a removed auth check, or request input newly
reaching SQL or a shell. Claude gets the list with file and line and must
restore each check or explain why the change is intended. Each signal is raised
once per session. Other sensitive changes, such as a dependency bump or an
edited workflow, stay in the review report and do not stop Claude. It also registers the
MCP server below and a `review-session` skill. Outside a Git repository the hooks
do nothing. Inside Git, unavailable review is reported in the hook diagnostics.

## Claude Code: wire the hooks by hand

Use the same maintained scripts as the plugin. From the repository root:

```bash
mkdir -p .claude/hooks
curl -fsSL -o .claude/hooks/repopilot-snapshot.sh https://raw.githubusercontent.com/MykytaStel/repopilot/main/integrations/claude-code/repopilot/scripts/snapshot.sh
curl -fsSL -o .claude/hooks/repopilot-guard.sh https://raw.githubusercontent.com/MykytaStel/repopilot/main/integrations/claude-code/repopilot/scripts/guard.sh
```

Merge these entries into `.claude/settings.json`, preserving any existing
hooks and settings:

```json
{
  "hooks": {
    "SessionStart": [
      {"hooks": [{"type": "command", "command": "sh .claude/hooks/repopilot-snapshot.sh", "timeout": 60}]}
    ],
    "Stop": [
      {"hooks": [{"type": "command", "command": "sh .claude/hooks/repopilot-guard.sh", "timeout": 180}]}
    ]
  }
}
```

The same unavailable-review diagnostics, integrity signals, and one-reprompt
limit apply. Use either the plugin or these project hooks so the loop runs once.

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
reviews everything the session changed. If the session weakened a check or a
safeguard (the same list as the Claude Code plugin), it sends the agent one
follow-up message listing each signal with its file and line, and asks the
agent to restore the check or explain why the change is intended. Each signal
is raised once per session. `loop_limit: 1` and the script's
own `loop_count` check keep it to one follow-up per stop. The hook prints `{}`
and does nothing when the turn was aborted, outside a Git repository, or when
review is unavailable (with the diagnostic on stderr).

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

## Gemini CLI: install the extension

With the `repopilot` CLI installed:

```bash
gemini extensions install https://github.com/MykytaStel/repopilot
```

The extension runs the same two scripts as the Claude Code plugin and
registers the MCP server for the current workspace. Gemini CLI asks you to
trust its hooks once.

`SessionStart` takes a snapshot. `AfterAgent` runs the review after each agent
reply. When it finds a weakened check, it rejects the reply once and sends the
list back, so the agent gets another turn to restore the check or explain the
change.

### Without the extension: project hooks

To keep the hooks in the repository instead, Gemini CLI reads them from
`.gemini/settings.json`. From the repository root:

```bash
mkdir -p .gemini/hooks
curl -fsSL -o .gemini/settings.json https://raw.githubusercontent.com/MykytaStel/repopilot/main/integrations/gemini/settings.json
curl -fsSL -o .gemini/hooks/repopilot-snapshot.sh https://raw.githubusercontent.com/MykytaStel/repopilot/main/integrations/claude-code/repopilot/scripts/snapshot.sh
curl -fsSL -o .gemini/hooks/repopilot-guard.sh https://raw.githubusercontent.com/MykytaStel/repopilot/main/integrations/claude-code/repopilot/scripts/guard.sh
```

If `.gemini/settings.json` already exists, merge the `hooks` and `mcpServers`
entries from [`integrations/gemini/settings.json`](../integrations/gemini/settings.json)
into it instead of overwriting it.

## Google Antigravity

An observed Gemini CLI 0.62.0 Google login was rejected because personal Code
Assist no longer supported that client. Follow Google's
[Gemini CLI migration guide](https://antigravity.google/docs/cli/gcli-migration/)
for the current Antigravity client rather than treating this as a RepoPilot
failure.

Register the local MCP server with Antigravity CLI:

```bash
agy mcp add repopilot -- repopilot mcp --root .
agy mcp list
```

Use [the manual snapshot/review loop](#any-agent-agentsmd) in each Git workspace.
This provides MCP tools; automatic stop behavior is not claimed for Antigravity.
Interactive MCP calls require the client's permission. For headless context
calls, merge `"mcp(repopilot/repopilot_context)"` into `permissions.allow` in
`~/.gemini/antigravity-cli/settings.json`, preserving existing rules. This
allows that context tool only; other tools keep their existing permissions.
Headless denials can still exit `0` with a `SUCCESS` status: check that the
requested MCP data actually returned and inspect stderr or `denied_actions`.
A legacy extension import can pin `${workspacePath}` to the directory used at
import time. Verify the resulting MCP root before using it in another project.

## GitHub Copilot coding agent

Copilot's coding agent works in GitHub Actions and opens a pull request. Its
hooks cannot stop the agent, so RepoPilot checks its work in two places:

1. Copy [`integrations/copilot/copilot-setup-steps.yml`](../integrations/copilot/copilot-setup-steps.yml)
   to `.github/workflows/copilot-setup-steps.yml`. The agent's environment then
   has the `repopilot` CLI and a snapshot before it starts.
   RepoPilot itself includes [this setup workflow](../.github/workflows/copilot-setup-steps.yml),
   pinned to the stable 0.25.0 package. Run it manually to verify installation
   and snapshot creation; this does not prove an agent PR or MCP configuration.
2. Open **Settings → Copilot → MCP servers** in your repository and add the server from
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
- **`behavioral.removed-export-still-imported`** — a named or direct default export an agent
  deleted or renamed while a local caller still imports the old name. Fires
  only when the resolver proves the caller targets the changed module; a
  coordinated rename across both sides produces no signal. Default aliases require
  a proven top-level local binding; forwarding, parser failures, namespace and
  dynamic imports remain explicit limits. CLI/MCP review and changed scans retain
  the same caller occurrence; full scans lack historical removal evidence. Rust
  covers removed public free functions through a plain child module declared
  in that caller; [language support](language-support.md#direct-rust-public-function-removal)
  lists the exact forms and abstention limits.

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
