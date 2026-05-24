# RepoPilot

[![Crates.io](https://img.shields.io/crates/v/repopilot.svg)](https://crates.io/crates/repopilot)
[![npm](https://img.shields.io/npm/v/repopilot.svg)](https://www.npmjs.com/package/repopilot)
[![CI](https://github.com/MykytaStel/repopilot/actions/workflows/ci.yaml/badge.svg)](https://github.com/MykytaStel/repopilot/actions)
[![GitHub Release](https://img.shields.io/github/v/release/MykytaStel/repopilot)](https://github.com/MykytaStel/repopilot/releases)
[![License](https://img.shields.io/crates/l/repopilot.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/MykytaStel/repopilot?style=social)](https://github.com/MykytaStel/repopilot)

**Local-first repository audits for safer human and AI-assisted code changes.**

RepoPilot scans a repository on your machine, ranks evidence-backed risks, helps
review pull requests, and produces local AI-ready context. It does not upload
source code, run a hosted scanner, call LLM APIs, or send telemetry.

RepoPilot is not a language linter replacement. It focuses on repository-level
signals that are easy to miss one file at a time: secrets, runtime footguns,
import graph risk, review blast radius, baseline adoption, and concise local
remediation context.

## Install

```bash
cargo install repopilot
npm install -g repopilot
brew tap mykytastel/repopilot && brew install repopilot
curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | bash
```

Build from source:

```bash
git clone https://github.com/MykytaStel/repopilot.git
cd repopilot
cargo build --release
```

More install options: [docs/install.md](docs/install.md).

## First 5 Minutes

```bash
repopilot scan .
repopilot doctor .
repopilot baseline create . --output .repopilot/baseline.json
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
repopilot review . --base origin/main
repopilot ai context . --budget 4k
```

That path answers:

- what is high-risk right now;
- whether the repo is ready for CI adoption;
- which findings are accepted existing debt;
- what the current branch changed;
- what local context to give an AI coding assistant.

Save audit evidence:

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot scan . --format json --output repopilot-report.json --receipt .repopilot/receipt.json
repopilot scan . --format sarif --output repopilot.sarif
```

## Trust Mode

The default scan profile is intentionally quiet. It keeps high-trust security,
runtime, and import-graph findings visible, while broad maintainability,
TODO/FIXME/HACK, long-function, complex-file, and testing-gap heuristics stay in
strict mode.

```bash
repopilot scan .                  # default profile
repopilot scan . --profile strict # full audit output
repopilot scan . --include-maintainability
```

Hidden suggestions are summarized by intent, rule, category, and reason, so the
default report stays quiet without pretending strict-only findings do not exist.

## Stable Command Surface

RepoPilot's public top-level commands stay focused before 1.0:

```text
scan | review | baseline | compare | doctor | inspect | ai | init | cache
```

New diagnostics should fit under existing commands rather than adding new
top-level workflows.

Common commands:

```bash
repopilot scan .
repopilot scan . --profile strict
repopilot review . --base origin/main
repopilot baseline create .
repopilot compare before.json after.json
repopilot inspect rules
repopilot inspect eval-rules --format json
repopilot ai context .
repopilot cache clear .
```

## Rule Quality

Rules move through `experimental -> preview -> stable`.

A rule can be `stable` only when it has true-positive and false-positive
fixtures, clean finding-contract output, metadata, false-positive notes, docs
for high/critical findings, and a clean `repopilot inspect eval-rules` result.

Before 0.20.0, RepoPilot prioritizes depth over breadth:

- secrets and private keys;
- Rust panic/runtime risk;
- JavaScript, Python, Go, JVM, and .NET runtime exits;
- import graph risks;
- review diff and blast-radius behavior.

## CI Gate

Minimal release gates:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
npm run test:npm
./scripts/smoke-product.sh
repopilot inspect eval-rules --format json
repopilot scan .
```

## Docs

| Document | Purpose |
|---|---|
| [Install](docs/install.md) | Cargo, npm, Homebrew, curl, and source builds |
| [Commands](docs/commands.md) | Task-oriented CLI workflows |
| [Configuration](docs/configuration.md) | `repopilot.toml`, presets, ignores, and baseline adoption |
| [Rulesets](docs/rulesets.md) | Built-in rules, lifecycle, severity, and confidence |
| [Reports](docs/reports.md) | JSON, SARIF, Markdown, HTML, receipts, and schema fields |
| [CI/GitHub](docs/integrations/github-code-scanning.md) | GitHub Actions and SARIF upload |
| [Roadmap](docs/roadmap.md) | 0.x product cuts, 0.13.x focus, and v1 gates |

## License

RepoPilot is licensed under **MIT OR Apache-2.0**.
