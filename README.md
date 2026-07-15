# RepoPilot

[![Crates.io](https://img.shields.io/crates/v/repopilot.svg)](https://crates.io/crates/repopilot)
[![npm](https://img.shields.io/npm/v/repopilot.svg)](https://www.npmjs.com/package/repopilot)
[![CI](https://github.com/MykytaStel/repopilot/actions/workflows/ci.yaml/badge.svg)](https://github.com/MykytaStel/repopilot/actions)
[![License](https://img.shields.io/crates/l/repopilot.svg)](LICENSE)

**Deterministic review for code you didn't write.**

Coding agents and busy teams produce more diffs than anyone can carefully
read. RepoPilot is a local Rust CLI that inspects a Git change and reports,
with structural evidence, where it crosses security boundaries, changes
behavior, and how far it reaches through the import graph. There is no LLM
inside and nothing leaves your machine: the same diff always produces the
same answer, so you can gate CI on it.

## Sixty seconds: an agent "optimizes" image uploads

An agent is asked to shell out to `mogrify` after image edits in a Django
app (Wagtail). The diff is one file, +12 −6, and it works. It also quietly
drops a permission check and pipes request input into a shell.

<p align="center">
  <img src="https://raw.githubusercontent.com/MykytaStel/repopilot/main/docs/demos/03-agent-review.gif" alt="repopilot review flagging a removed auth check and a request-to-shell taint flow in a one-file diff" width="800">
</p>

One command, `repopilot review .`, and the diff answers for itself:

```text
⚑ access control changed — wagtail/images/views/images.py
⚑ auth check removed — wagtail/images/views/images.py:264
    Authentication/authorization check removed (auth calls: 1 -> 0)
⚑ subprocess/exec added — wagtail/images/views/images.py:270
⚑ untrusted input reaches subprocess/exec — wagtail/images/views/images.py:270
    HTTP request input reaches subprocess/exec: subprocess.run("mogrify -quality " + quality + ...
⚠ A code boundary changed but no test did — confirm it's still covered.
```

The edit is scripted so you can replay it on the pinned Wagtail checkout
RepoPilot uses for regression testing — but it is the kind of diff agents
ship every day:

```bash
python3 scripts/zoo.py clone --only wagtail
scripts/demo-agent-edit.sh .zoo/wagtail
repopilot review .zoo/wagtail
```

> RepoPilot reports structural evidence, not a security verdict. A flagged
> flow is a path to verify, not a confirmed vulnerability. Use it beside
> tests, linters, and dedicated security tools.

## Why not an LLM reviewer?

- **Deterministic.** Same diff in, same signals out. You can gate a pipeline
  on it and reproduce any result; there is no prompt to drift and no model
  to update under you.
- **Local.** No source upload, no API key, no per-review cost, no telemetry.
  It runs offline, including the MCP server.
- **Evidence, not opinions.** Every signal points at a line and states the
  structural fact behind it — an auth call count that went down, a request
  value reaching `subprocess.run`. Nothing "looks fine."

LLM reviewers are useful; a deterministic layer under them is what makes
their output checkable.

## Install

```bash
cargo install repopilot
# or
npm install -g repopilot
```

Homebrew, GitHub Releases, and source builds:
[Installation](https://github.com/MykytaStel/repopilot/blob/main/docs/install.md).

## Review a change

```bash
repopilot review .                        # working tree vs HEAD
repopilot review . --base origin/main     # branch vs main
```

Review groups evidence into tiers: security boundaries (access control,
request trust, deploy surface, supply chain, secrets), behavioral changes
(network, subprocess, filesystem, SQL, removed error handling or auth),
algorithmic shifts, taint-lite flows (changed request/process input reaching
SQL, exec, filesystem-write, or network sinks), and blast radius (files that
import what changed).

Signals are advisory by default. Gate CI on the high-confidence tier:

```bash
repopilot review . --base origin/main --fail-on-review definitely
```

## Guard an agent run

Take a snapshot before the agent starts, review everything it did — commits
and uncommitted edits — when it stops:

```bash
repopilot snapshot
# ... agent session ...
repopilot review --since-snapshot
```

To wire this in permanently — a Claude Code hook that reviews every agent
session, an MCP server the agent can query mid-task (`repopilot init
--mcp-client claude|cursor|generic`), or a PR gate via the GitHub Action —
see [Guard your agent runs](https://github.com/MykytaStel/repopilot/blob/main/docs/agent-guardrail.md).

## Beyond the diff

- `repopilot scan .` — full-repository audit (architecture, coupling,
  framework, testing). `repopilot baseline create .` adopts existing debt so
  only new findings gate.
- `repopilot ai context .` — one compact Markdown handoff of repository
  facts, findings, and a prioritized fix plan for an external assistant.
  Local, no LLM calls.
- `repopilot mcp --root .` — the same analysis over stdio for coding agents:
  synchronous, root-confined, offline.

Details: [Common workflows](https://github.com/MykytaStel/repopilot/blob/main/docs/commands.md).

## Documentation

- [Documentation index](https://github.com/MykytaStel/repopilot/blob/main/docs/README.md)
- [Guard your agent runs](https://github.com/MykytaStel/repopilot/blob/main/docs/agent-guardrail.md)
- [CLI reference](https://github.com/MykytaStel/repopilot/blob/main/docs/cli.md)
- [Configuration](https://github.com/MykytaStel/repopilot/blob/main/docs/configuration.md)
- [Reports and schemas](https://github.com/MykytaStel/repopilot/blob/main/docs/reports.md)
- [GitHub pull request integration](https://github.com/MykytaStel/repopilot/blob/main/docs/integrations/github-code-scanning.md)

Contributing and development commands: [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT OR Apache-2.0.
