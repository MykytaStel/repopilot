# RepoPilot

[![Crates.io](https://img.shields.io/crates/v/repopilot.svg)](https://crates.io/crates/repopilot)
[![npm](https://img.shields.io/npm/v/repopilot.svg)](https://www.npmjs.com/package/repopilot)
[![CI](https://github.com/MykytaStel/repopilot/actions/workflows/ci.yaml/badge.svg)](https://github.com/MykytaStel/repopilot/actions)
[![GitHub Release](https://img.shields.io/github/v/release/MykytaStel/repopilot)](https://github.com/MykytaStel/repopilot/releases)
[![License](https://img.shields.io/crates/l/repopilot.svg)](LICENSE)

**Review what you or an AI agent changed before you merge.**

RepoPilot is a fast, local-first Rust CLI for Git change review. It flags
security-boundary changes, behavioral and algorithmic shifts, taint-lite flows,
and blast radius so maintainers can focus on the parts of a diff that deserve
extra attention. It is deterministic, runs entirely on your machine, and can be
called by coding agents over MCP. Nothing is uploaded.

```text
git diff -> boundary + behavior + taint + blast radius -> review or CI gate
```

<p align="center">
  <img src="https://raw.githubusercontent.com/MykytaStel/repopilot/main/docs/demos/02-review.gif" alt="RepoPilot reviewing a branch change" width="800">
</p>

> RepoPilot reports structural evidence, not a security verdict. Use it beside
> tests, linters, type checkers, and dedicated security tools.

## Install

```bash
cargo install repopilot
# or
npm install -g repopilot
```

Homebrew, curl, GitHub Releases, and source builds are documented in
[Installation](https://github.com/MykytaStel/repopilot/blob/main/docs/install.md).

## Choose Your Workflow

| Goal | Command |
|---|---|
| Review what changed before merge | `repopilot review .` |
| Review a branch against main | `repopilot review . --base origin/main` |
| Audit the whole repository | `repopilot scan .` |
| Adopt existing debt gradually | `repopilot baseline create .` |
| Review a complete agent run | `repopilot snapshot` then `repopilot review --since-snapshot` |
| Prepare context for an assistant | `repopilot ai context .` |
| Connect a local MCP client | `repopilot mcp --root .` |

`review` is the daily change-review workflow. Use `scan` when repository-wide
architecture, framework, testing, and coupling results must be authoritative.

## Review A Change

Review the working tree against `HEAD`:

```bash
repopilot review .
```

Review a branch before merge:

```bash
repopilot review . --base origin/main
```

RepoPilot groups review evidence into confidence tiers:

- security boundaries: access control, request trust, deploy surface, supply
  chain, and secret configuration;
- behavioral changes: network, subprocess, filesystem, SQL, dependencies,
  migrations, removed error handling, or removed auth checks;
- algorithmic changes: deeper nesting, nested loops, growth, or recursion;
- taint-lite flows: changed request or process input reaching SQL, exec,
  filesystem-write, or outbound-network sinks;
- blast radius: files that import a changed file.

All review signals ship at `preview`. They are advisory by default; enable the
explicit high-confidence gate with:

```bash
repopilot review . --base origin/main --fail-on-review definitely
```

For a complete agent run:

```bash
repopilot snapshot
# let the agent or developer work
repopilot review --since-snapshot
```

## Full Repository Audit

The broader scan remains available for repository adoption and CI:

```bash
repopilot scan .
repopilot baseline create .
repopilot scan . \
  --baseline .repopilot/baseline.json \
  --fail-on new-high
```

Default scans hide low-confidence, experimental, and broad maintainability
suggestions. Use `--profile strict` for the full audit surface.

## AI Handoff

When you want an external assistant to drive a fix, `repopilot ai context` turns a
scan into one compact, copy-paste-ready Markdown handoff — locally, with no network
or LLM calls:

```bash
repopilot ai context .
repopilot ai context . --focus security --budget 8k
repopilot ai context . --no-task --output ai-context.md   # fact-only, for embedding
```

The handoff bundles everything an assistant needs in one document: repository facts
and risk, the findings with evidence, a prioritized **P0–P3 remediation plan** with
the Context Risk Graph edit order, working rules, and a verification checklist. Three
controls shape it:

- `--focus` — `security`, `arch`, `quality`, `framework`, or `all` (default);
- `--budget` — `2k`/`4k`/`8k`/`16k` or an integer token target (default `4k`,
  roughly four characters per token), so the output fits your model's context;
- `--output FILE` — write Markdown to a file instead of stdout (or pipe to your
  clipboard, e.g. `| pbcopy`). Pass `--no-task` to drop the agent guidance and emit
  fact-only context.

## MCP Server

`repopilot mcp` exposes that context — fact-only, the way `--no-task` emits it —
plus review, scan, file explanation, and finding replay tools over stdio so
coding agents can call it directly:

```bash
claude mcp add repopilot -- repopilot mcp --root .
```

The MCP server is synchronous, root-confined, and makes no network or LLM calls.
See [MCP server](https://github.com/MykytaStel/repopilot/blob/main/docs/mcp.md).

## Distribution

Official CLI channels are crates.io, npm, Homebrew, and GitHub Releases.
Editor extensions and PyPI packages are not supported distribution channels.

## Documentation

- [Documentation index](https://github.com/MykytaStel/repopilot/blob/main/docs/README.md)
- [Common workflows](https://github.com/MykytaStel/repopilot/blob/main/docs/commands.md)
- [CLI reference](https://github.com/MykytaStel/repopilot/blob/main/docs/cli.md)
- [Configuration](https://github.com/MykytaStel/repopilot/blob/main/docs/configuration.md)
- [Reports and schemas](https://github.com/MykytaStel/repopilot/blob/main/docs/reports.md)
- [GitHub pull request integration](https://github.com/MykytaStel/repopilot/blob/main/docs/integrations/github-code-scanning.md)
- [Signal contract](https://github.com/MykytaStel/repopilot/blob/main/docs/engineering/signal-contract.md)
- [Release process](https://github.com/MykytaStel/repopilot/blob/main/docs/release.md)

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
npm run release:contract
./scripts/smoke-product.sh
```

## License

RepoPilot is licensed under **MIT OR Apache-2.0**.
