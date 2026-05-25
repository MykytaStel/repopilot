# RepoPilot

[![Crates.io](https://img.shields.io/crates/v/repopilot.svg)](https://crates.io/crates/repopilot)
[![npm](https://img.shields.io/npm/v/repopilot.svg)](https://www.npmjs.com/package/repopilot)
[![CI](https://github.com/MykytaStel/repopilot/actions/workflows/ci.yaml/badge.svg)](https://github.com/MykytaStel/repopilot/actions)
[![GitHub Release](https://img.shields.io/github/v/release/MykytaStel/repopilot)](https://github.com/MykytaStel/repopilot/releases)
[![License](https://img.shields.io/crates/l/repopilot.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/MykytaStel/repopilot?style=social)](https://github.com/MykytaStel/repopilot)

**Local-first repository audits for safer human and AI-assisted code changes.**

RepoPilot is a fast Rust CLI that scans your repository locally, ranks evidence-backed risks, helps review pull requests, supports baseline adoption, and generates AI-ready context without uploading your source code.

It is built for maintainers who want practical repository-level signal, not another noisy linter dump.

```text
local scan -> evidence-backed findings -> risk-ranked review -> baseline adoption -> AI-ready local context -> CI gate
```

## Why RepoPilot exists

Modern teams move fast, but codebases quietly accumulate risks that are easy to miss one file at a time:

- committed secrets and unsafe configuration files;
- runtime footguns such as production exits and panic paths;
- import graph risks and architectural coupling;
- noisy maintainability issues that should not block every PR;
- large review blast radius;
- old findings that need a baseline instead of endless repeated warnings;
- AI coding sessions that need clean local context instead of random pasted files.

RepoPilot focuses on repository-level evidence and review workflows.

It does **not** replace language linters, formatters, type checkers, or security scanners. It complements them by connecting findings to repository structure, change risk, review context, and local remediation workflows.

## Local-first by design

RepoPilot does not:

- upload your source code;
- run a hosted scanner;
- call LLM APIs implicitly;
- send telemetry;
- require a SaaS account.

AI-related commands are local formatting/context helpers. They package audit evidence for tools like Claude Code, ChatGPT, Cursor, Zed, or another assistant, but RepoPilot itself stays local-first.

## What RepoPilot checks

RepoPilot is intentionally conservative in the default profile. It prioritizes high-trust findings that are useful during daily development and CI review.

Core focus areas:

| Area | Examples |
|---|---|
| Security | secret candidates, private keys, committed env files |
| Runtime risk | Rust panic/unwrap paths, JavaScript/Python/Go/JVM/.NET runtime exits |
| Architecture | circular dependencies, excessive fan-out, instability hubs |
| Review risk | changed files, blast radius, branch-vs-base review context |
| Baseline adoption | distinguish new risk from accepted existing debt |
| Reports | JSON, Markdown, SARIF, receipts, report envelopes |
| AI context | local, budgeted, evidence-backed context for coding assistants |

Broad maintainability signals such as long files, long functions, TODO/FIXME/HACK markers, complex files, and testing gaps are available in strict mode instead of dominating default output.

## Install

Choose one install method.

### Cargo

```bash
cargo install repopilot
```

### npm

```bash
npm install -g repopilot
```

### Homebrew

```bash
brew tap mykytastel/repopilot
brew install repopilot
```

### Linux/macOS installer

```bash
curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | bash
```

### Build from source

```bash
git clone https://github.com/MykytaStel/repopilot.git
cd repopilot
cargo build --release
```

More install options: [docs/install.md](docs/install.md).

## First 5 minutes

Run a default local scan:

```bash
repopilot scan .
```

Check whether the repository is ready for adoption:

```bash
repopilot doctor .
```

Create a baseline for existing debt:

```bash
repopilot baseline create . --output .repopilot/baseline.json
```

Fail only on new high-risk findings:

```bash
repopilot scan . \
  --baseline .repopilot/baseline.json \
  --fail-on new-high
```

Review your current branch against `main`:

```bash
repopilot review . --base origin/main
```

Generate local AI-ready context:

```bash
repopilot ai context . --budget 4k
```

Save audit evidence:

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot scan . --format json --output repopilot-report.json --receipt .repopilot/receipt.json
repopilot scan . --format sarif --output repopilot.sarif
```

## Core workflows

### 1. Scan the repository

```bash
repopilot scan .
```

Use this for day-to-day local checks and CI-friendly repository audits.

### 2. Run a strict audit

```bash
repopilot scan . --profile strict
```

Use strict mode for cleanup passes, refactoring work, rule development, and release-hardening audits.

### 3. Review a branch

```bash
repopilot review . --base origin/main
```

Use this before opening or merging a PR to understand changed files, review risk, and branch-specific findings.

### 4. Adopt RepoPilot gradually

```bash
repopilot baseline create . --output .repopilot/baseline.json
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
```

This lets a mature repository start using RepoPilot without failing CI on old accepted debt.

### 5. Generate local AI context

```bash
repopilot ai context . --budget 4k
```

Use this when working with an AI coding assistant. RepoPilot produces concise local context based on repository evidence instead of asking you to paste random files manually.

## Trust Mode

RepoPilot's default profile is intentionally quiet.

The default scan answers:

- what can break production;
- what can leak secrets;
- what should block a release;
- what is safe to review later;
- what belongs in a strict/deep audit workflow.

```bash
repopilot scan .                  # default profile
repopilot scan . --profile strict # full audit output
repopilot scan . --include-maintainability
```

Default profile keeps high-trust security, runtime, and import-graph findings visible.

Strict profile preserves the full raw audit output, including broad maintainability and testing heuristics.

Hidden suggestions are summarized instead of silently dropped, so the default report stays quiet without becoming opaque.

More: [docs/trust-mode.md](docs/trust-mode.md).

## Stable command surface

RepoPilot keeps the public top-level CLI small before 1.0:

```text
scan | review | baseline | compare | doctor | inspect | ai | init | cache
```

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

New diagnostics should fit under existing commands instead of adding more top-level workflows.

## Rule quality contract

RepoPilot rules move through a lifecycle:

```text
experimental -> preview -> stable
```

A rule should become stable only when it has:

- complete rule metadata;
- true-positive fixtures;
- false-positive fixtures;
- deterministic stable finding IDs;
- evidence pointing to concrete files, lines, or scopes;
- false-positive notes;
- docs URL for high/critical findings;
- clean local validation through `inspect eval-rules`.

Run the rule quality gate:

```bash
repopilot inspect eval-rules --format json
```

Evaluate one rule:

```bash
repopilot inspect eval-rules --rule security.env-file-committed --format json
```

The gate reports fixture coverage, missing findings, unexpected findings, contract violations, stable ID failures, and quality gate failures.

More: [docs/rule-quality-gate.md](docs/rule-quality-gate.md).

## CI example

Minimal CI gate:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
npm run test:npm
./scripts/smoke-product.sh
repopilot inspect eval-rules --format json
repopilot scan .
```

GitHub Actions with SARIF upload:

```bash
repopilot scan . --format sarif --output repopilot.sarif
```

More: [docs/integrations/github-code-scanning.md](docs/integrations/github-code-scanning.md).

## Output formats

RepoPilot can produce:

| Format | Use case |
|---|---|
| Console | fast local feedback |
| Markdown | PR comments, human-readable reports |
| JSON | automation and internal tooling |
| SARIF | GitHub code scanning |
| HTML | shareable local reports |
| Receipt | reproducible scan metadata |

More: [docs/reports.md](docs/reports.md).

## Configuration

RepoPilot can be configured with `repopilot.toml`.

Use configuration for:

- ignores;
- presets;
- rule tuning;
- baseline adoption;
- scan behavior;
- report settings.

More: [docs/configuration.md](docs/configuration.md).

## Project status

RepoPilot is pre-1.0.

The current product direction is to strengthen trust, fixture-backed rules, baseline adoption, review workflows, and local-first AI context before expanding into too many rule families.

The pre-1.0 release line prioritizes:

- trustworthy default output;
- stable command surface;
- strong rule lifecycle;
- clean self-audit behavior;
- verified distribution through crates.io, npm, GitHub Releases, Homebrew, and installer scripts.

Release-specific plans, checklists, and historical notes live outside the README so this page can stay useful across future versions.

More: [docs/roadmap.md](docs/roadmap.md).

## Documentation

| Document | Purpose |
|---|---|
| [Install](docs/install.md) | Cargo, npm, Homebrew, curl, and source builds |
| [Commands](docs/commands.md) | Task-oriented CLI workflows |
| [Configuration](docs/configuration.md) | `repopilot.toml`, presets, ignores, and baseline adoption |
| [Rulesets](docs/rulesets.md) | Built-in rules, lifecycle, severity, and confidence |
| [Trust Mode](docs/trust-mode.md) | Default vs strict visibility policy |
| [Rule lifecycle](docs/rule-lifecycle.md) | Experimental, preview, and stable rule expectations |
| [Rule quality gate](docs/rule-quality-gate.md) | Fixture-backed stable/default-visible rule contract |
| [Reports](docs/reports.md) | JSON, SARIF, Markdown, HTML, receipts, and schema fields |
| [CI/GitHub](docs/integrations/github-code-scanning.md) | GitHub Actions and SARIF upload |
| [Roadmap](docs/roadmap.md) | Pre-1.0 product direction, release themes, and v1 gates |

## Development

Run local checks:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
npm run test:npm
```

Run product smoke checks:

```bash
./scripts/smoke-product.sh
```

Run a self-scan:

```bash
cargo run -- scan .
```

Run rule quality evaluation from source:

```bash
cargo run -- inspect eval-rules --format json
```

## Contributing

RepoPilot values small, focused changes.

Good contributions include:

- fixture-backed rule improvements;
- false-positive reductions;
- clearer evidence in findings;
- better baseline/review ergonomics;
- docs that make local adoption easier;
- tests that protect rule precision and output contracts.

Before marking a rule stable or default-visible, make sure it has true-positive and false-positive fixtures, metadata, false-positive notes, deterministic output, and a clean `inspect eval-rules` result.

## License

RepoPilot is licensed under **MIT OR Apache-2.0**.
