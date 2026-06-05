# RepoPilot

[![Crates.io](https://img.shields.io/crates/v/repopilot.svg)](https://crates.io/crates/repopilot)
[![npm](https://img.shields.io/npm/v/repopilot.svg)](https://www.npmjs.com/package/repopilot)
[![CI](https://github.com/MykytaStel/repopilot/actions/workflows/ci.yaml/badge.svg)](https://github.com/MykytaStel/repopilot/actions)
[![GitHub Release](https://img.shields.io/github/v/release/MykytaStel/repopilot)](https://github.com/MykytaStel/repopilot/releases)
[![Downloads](https://img.shields.io/crates/d/repopilot)](https://crates.io/crates/repopilot)
[![License](https://img.shields.io/crates/l/repopilot.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/MykytaStel/repopilot?style=social)](https://github.com/MykytaStel/repopilot)

**See what your AI agent — or you — just changed, before you merge.**

RepoPilot is a fast, local-first Rust CLI that reviews a Git change and flags when it crossed a **security boundary** — the parts of your repo that decide *who can do what* (auth, sessions, permissions, CORS) and *how it ships* (CI, Dockerfiles, dependencies, committed `.env`) — then shows how far each changed file reaches. It's deterministic, runs entirely on your machine, and plugs into your AI coding agent over [MCP](#use-with-ai-agents-mcp). Nothing is uploaded.

```text
review a change -> boundary + blast radius -> baseline / CI gate -> local AI context
```

<p align="center">
  <img src="docs/demos/02-review.gif" alt="RepoPilot reviewing a branch change" width="800">
</p>

> RepoPilot flags *that* a boundary moved and how far it reaches — it does **not** prove the change is safe. Think "open the report before merging," not "security audit." It complements linters, type checkers, and dedicated security scanners; it doesn't replace them.

## Install

```bash
cargo install repopilot     # or: npm install -g repopilot
```

Homebrew, install script, and from-source: [docs/install.md](docs/install.md).

## Quick start

Review a change — what you'd check before merging, and what an agent can check on its own edits:

```bash
repopilot review . --base main
```

```text
$ repopilot review . --base main

Changed files: 2
In-diff findings: 2

Blast radius:
  The following files import changed files and may need extra review:
  - src/admin.ts
  - src/routes.ts

Security boundary changed [preview]:
  ⚑ access control  src/middleware/auth.ts  (imported by 2 files)
  ⚑ request trust   src/server/cors.ts
  ⚠ A code boundary changed but no test did — confirm it's still covered.
```

Boundary categories: **access control**, **request trust**, **deploy surface**, **supply chain**, **secret config**. Tune or disable them in `repopilot.toml` under `[security_boundary]` (ships at `preview`).

Full audit, a CI gate that fails only on *new* risk, and a local brief for an assistant:

```bash
repopilot scan .                                                         # full local audit (quiet by default)
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high  # fail CI only on newly introduced risk
repopilot ai context . --budget 4k                                       # budgeted, evidence-backed Markdown
```

First five minutes for AI-assisted work:

```bash
repopilot doctor .          # confirm RepoPilot is ready for this repo
repopilot scan .            # see current visible risk
repopilot ai context .      # prepare local, evidence-backed context
repopilot ai plan .         # get a prioritized remediation plan
repopilot ai prompt .       # generate a paste-ready prompt for an assistant
repopilot review .          # check the assistant's change before merging
```

## Use with AI agents (MCP)

RepoPilot ships a local [Model Context Protocol](https://modelcontextprotocol.io) server so AI coding agents (Claude Code, Cursor, …) can call it as a tool — the deterministic, private check an agent can run on its **own** edits before handing you the PR.

```bash
claude mcp add repopilot -- repopilot mcp
```

It runs over stdio (JSON-RPC, no network, no AI calls) and exposes four tools:

- `repopilot_review_change` — audit the current Git changes: in-diff vs out-of-diff findings, security-boundary signals, and blast radius (structured JSON).
- `repopilot_scan` — full repository audit as JSON.
- `repopilot_context` — a budgeted, AI-ready Markdown brief of the repo.
- `repopilot_explain_file` — how a single file is classified and which rules apply.

More: [docs/mcp.md](docs/mcp.md).

## What RepoPilot does

| Capability | What it does |
|---|---|
| **Review a change** | findings on changed lines, blast radius, and security-boundary signals — for you or your agent (`review`, MCP) |
| Full scan | repo-wide, evidence-ranked findings — secrets, runtime footguns, architecture — quiet by default ([trust mode](docs/trust-mode.md)) |
| Baseline + CI gate | accept current debt as a baseline; fail CI only on newly introduced risk |
| Reports | Console, Markdown, JSON, [SARIF](docs/integrations/github-code-scanning.md), HTML, receipts |
| AI context | local, budgeted, evidence-backed Markdown brief for coding assistants |

Rules carry an `experimental -> preview -> stable` lifecycle and a quality gate (fixtures, stable IDs, evidence). See [docs/rule-quality-gate.md](docs/rule-quality-gate.md).

## Local-first

RepoPilot does not upload source code, run a hosted scanner, call LLM APIs implicitly, send telemetry, or require an account. AI commands only format local scan evidence as Markdown for tools such as Claude Code, Cursor, or Zed.

## Documentation

| Document | Purpose |
|---|---|
| [Install](docs/install.md) | Cargo, npm, Homebrew, curl, and source builds |
| [Commands](docs/commands.md) / [CLI](docs/cli.md) | Task-oriented workflows and the full flag reference |
| [Configuration](docs/configuration.md) | `repopilot.toml`, presets, ignores, baselines |
| [Rulesets](docs/rulesets.md) / [Rule quality gate](docs/rule-quality-gate.md) | Built-in rules, lifecycle, and the gate |
| [Trust Mode](docs/trust-mode.md) | Default vs strict visibility |
| [Reports](docs/reports.md) / [Code Scanning](docs/integrations/github-code-scanning.md) | Formats, schema, and SARIF in CI |
| [MCP](docs/mcp.md) | Using RepoPilot from AI agents |
| [Release Process](docs/release.md) | Local and CI release gates |

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
./scripts/smoke-product.sh
```

## License

RepoPilot is licensed under **MIT OR Apache-2.0**.
