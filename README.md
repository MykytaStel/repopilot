# RepoPilot

[![Crates.io](https://img.shields.io/crates/v/repopilot.svg)](https://crates.io/crates/repopilot)
[![npm](https://img.shields.io/npm/v/repopilot.svg)](https://www.npmjs.com/package/repopilot)
[![CI](https://github.com/MykytaStel/repopilot/actions/workflows/ci.yaml/badge.svg)](https://github.com/MykytaStel/repopilot/actions)
[![GitHub Release](https://img.shields.io/github/v/release/MykytaStel/repopilot)](https://github.com/MykytaStel/repopilot/releases)
[![License](https://img.shields.io/crates/l/repopilot.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/MykytaStel/repopilot?style=social)](https://github.com/MykytaStel/repopilot)

**Local-first repository audits for safer human and AI-assisted code changes.**

RepoPilot is a fast Rust CLI that scans a repository locally, ranks evidence-backed risks, supports baseline adoption, reviews branches, and creates AI-ready remediation context without uploading source code.

```text
scan -> review risk -> baseline adoption -> CI evidence -> local AI context
```

RepoPilot is not a replacement for language linters, formatters, type checkers, or dedicated security scanners. It complements them with repository-level evidence: secrets, runtime footguns, architecture risk, review blast radius, baseline status, and report formats that work in CI.

## Install

```bash
cargo install repopilot
npm install -g repopilot
brew tap mykytastel/repopilot && brew install repopilot
```

Installer:

```bash
curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | bash
```

From source:

```bash
git clone https://github.com/MykytaStel/repopilot.git
cd repopilot
cargo build --release
```

More: [docs/install.md](docs/install.md).

## First Run

```bash
repopilot scan .
repopilot doctor .
```

Adopt RepoPilot in an existing repository without failing CI on old debt:

```bash
repopilot baseline create . --output .repopilot/baseline.json
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
```

Review a branch:

```bash
repopilot review . --base origin/main
```

Generate local AI context:

```bash
repopilot ai context . --budget 4k
```

Save CI evidence:

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot scan . --format json --output repopilot-report.json --receipt .repopilot/receipt.json
repopilot scan . --format sarif --output repopilot.sarif
```

## What It Checks

| Area | Examples |
|---|---|
| Security | secret candidates, private keys, committed `.env` files |
| Runtime risk | Rust panic/unwrap paths, JavaScript/Python/Go/JVM/.NET process exits |
| Architecture | circular dependencies, excessive fan-out, instability hubs |
| Review risk | changed files, branch context, blast radius |
| Adoption | baselines, new-vs-existing findings, CI gates |
| Evidence | JSON, Markdown, SARIF, receipts, report envelopes |
| AI context | local, budgeted, evidence-backed Markdown for coding assistants |

## Trust Mode

Default scans are intentionally quiet. They keep high-trust security, runtime, and import-graph findings visible while summarizing broad maintainability and testing suggestions as strict-only noise.

```bash
repopilot scan .                  # compact default output
repopilot scan . --output-style full
repopilot scan . --profile strict # full audit output
```

Use strict mode for cleanup passes, rule development, and release hardening. Use the default profile for day-to-day local checks and CI gates.

More: [docs/trust-mode.md](docs/trust-mode.md).

## Core Commands

```text
scan | review | baseline | compare | doctor | inspect | ai | init | cache
```

Common workflows:

```bash
repopilot scan .
repopilot review . --base origin/main
repopilot baseline create .
repopilot compare before.json after.json
repopilot inspect rules
repopilot inspect eval-rules --format json
repopilot ai context .
```

More: [docs/commands.md](docs/commands.md) and [docs/cli.md](docs/cli.md).

## Baseline Adoption

A baseline records current findings as accepted existing debt. Future scans can fail only on newly introduced risk:

```bash
repopilot baseline create . --output .repopilot/baseline.json
git add .repopilot/baseline.json
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
```

Review baseline updates like code changes. A baseline is not a suppression file; it is a CI adoption contract.

## Rule Quality

Rules move through a lifecycle:

```text
experimental -> preview -> stable
```

Before a rule becomes stable or default-visible, it should have metadata, true-positive and false-positive fixtures, stable finding IDs, concrete evidence, false-positive notes, recommendations, and clean local evaluation:

```bash
repopilot inspect eval-rules --format json
```

More: [docs/rule-quality-gate.md](docs/rule-quality-gate.md).

## Local-First

RepoPilot does not upload source code, run a hosted scanner, call LLM APIs implicitly, send telemetry, or require a SaaS account. AI commands only format local scan evidence as Markdown for tools such as Claude Code, ChatGPT, Cursor, Zed, or another assistant.

## Reports

| Format | Use case |
|---|---|
| Console | fast local feedback |
| Markdown | PR comments and human-readable reports |
| JSON | automation and internal tooling |
| SARIF | GitHub Code Scanning |
| HTML | shareable local reports |
| Receipt | compact release/CI evidence |

More: [docs/reports.md](docs/reports.md).

## CI

Minimal baseline-aware gate:

```bash
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
```

GitHub Code Scanning:

```bash
repopilot scan . --format sarif --output repopilot.sarif
```

Release smoke:

```bash
./scripts/smoke-product.sh
```

More: [docs/integrations/github-code-scanning.md](docs/integrations/github-code-scanning.md).

## Documentation

| Document | Purpose |
|---|---|
| [Install](docs/install.md) | Cargo, npm, Homebrew, curl, and source builds |
| [Commands](docs/commands.md) | Task-oriented workflows |
| [CLI](docs/cli.md) | Complete command and flag reference |
| [Configuration](docs/configuration.md) | `repopilot.toml`, presets, ignores, and baselines |
| [Rulesets](docs/rulesets.md) | Built-in rules, lifecycle, severity, and confidence |
| [Trust Mode](docs/trust-mode.md) | Default vs strict visibility policy |
| [Reports](docs/reports.md) | JSON, SARIF, Markdown, HTML, receipts, and schema fields |
| [Release Process](docs/release.md) | Local and CI release gates |

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
npm run test:npm
./scripts/smoke-product.sh
```

Run rule evaluation from source:

```bash
cargo run -- inspect eval-rules --format json
```

## License

RepoPilot is licensed under **MIT OR Apache-2.0**.
