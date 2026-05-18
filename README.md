# RepoPilot

[![Crates.io](https://img.shields.io/crates/v/repopilot.svg)](https://crates.io/crates/repopilot)
[![npm](https://img.shields.io/npm/v/repopilot.svg)](https://www.npmjs.com/package/repopilot)
[![CI](https://github.com/MykytaStel/repopilot/actions/workflows/ci.yaml/badge.svg)](https://github.com/MykytaStel/repopilot/actions)
[![GitHub Release](https://img.shields.io/github/v/release/MykytaStel/repopilot)](https://github.com/MykytaStel/repopilot/releases)
[![License](https://img.shields.io/crates/l/repopilot.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/MykytaStel/repopilot?style=social)](https://github.com/MykytaStel/repopilot)

**Local-first repo intelligence for safer human and AI-assisted code changes.**

RepoPilot scans a repository locally, explains architecture/security/code-quality
risks, and turns findings into AI-ready context without uploading your source
code to a hosted scanner.

It is not meant to replace language linters. RepoPilot focuses on repository-level
signals: risky files, architecture drift, security candidates, testing gaps,
review risk, baselines, and remediation context for Claude Code, ChatGPT, Cursor,
or another coding assistant.

---

## Why RepoPilot?

Most tools either lint one language or send results into a dashboard. RepoPilot is
designed for the local development loop:

```text
scan -> understand risk -> generate AI context -> fix -> review
```

| Capability | Language linters | Hosted scanners | RepoPilot |
|---|---:|---:|---:|
| Runs locally | ✅ | ❌ | ✅ |
| No source upload required | ✅ | ❌ | ✅ |
| Cross-language repository view | ❌ | ✅ | ✅ |
| Architecture and repo risk signals | partial | ✅ | ✅ |
| AI-ready remediation context | ❌ | ❌ | ✅ |
| Baseline and PR review workflow | partial | ✅ | ✅ |
| Default noise reduction / Trust Mode | ❌ | partial | ✅ |

---

## Install

With Cargo:

```bash
cargo install repopilot
```

With npm:

```bash
npm install -g repopilot
```

With Homebrew:

```bash
brew tap mykytastel/repopilot
brew install repopilot
```

With curl on Linux/macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | bash
```

Build from source:

```bash
git clone https://github.com/MykytaStel/repopilot.git
cd repopilot
cargo build --release
```

Upgrade:

```bash
cargo install repopilot --force
npm update -g repopilot
brew update && brew upgrade repopilot
```

---

## Quick Start

Scan a repository:

```bash
repopilot scan .
```

Generate AI-ready context:

```bash
repopilot ai context .
```

Copy AI context on macOS:

```bash
repopilot ai context . | pbcopy
```

Save a Markdown report:

```bash
repopilot scan . --format markdown --output repopilot-report.md
```

Run a focused security scan:

```bash
repopilot scan . --min-severity high --rule security.secret-candidate
```

Review only changed code against a base branch:

```bash
repopilot review . --base origin/main
```

---

## Trust Mode

RepoPilot's default scan profile is intentionally quiet.

The default profile focuses on actionable production, security, and runtime risks.
Strict mode shows the full raw audit output, including maintainability and testing
suggestions.

```bash
repopilot scan .                  # default profile
repopilot scan . --profile strict # full audit output
repopilot scan . --include-maintainability
```

The visibility engine classifies findings by product intent:

```text
SecurityRisk | RuntimeRisk | ActionableRisk | Maintainability | TestingGap | Informational
```

Hidden suggestions are summarized by intent, rule, category, and reason, so the
default report stays quiet without silently losing information.

Read more:

- [Trust Mode](docs/trust-mode.md)
- [Core Visibility Engine](docs/core-visibility-engine.md)

---

## AI Workflow

RepoPilot does not call LLM APIs. It produces local Markdown that you choose where
to paste.

```bash
repopilot ai context . --focus security
repopilot ai plan . --focus security
repopilot ai prompt . --budget 8k
```

Typical loop:

```text
1. Run RepoPilot locally.
2. Read the risk summary.
3. Generate focused AI context.
4. Paste the context into Claude Code, ChatGPT, Cursor, or another assistant.
5. Apply the patch.
6. Run RepoPilot review/scan again.
```

See [AI workflows](docs/ai-workflows.md).

---

## Core Commands

| Command | Alias | Description |
|---|---:|---|
| `repopilot scan <path>` | `s` | Scan a project, folder, or file |
| `repopilot scan <path> --changed` | — | Scan changed files with local cache |
| `repopilot cache clear [path]` | — | Remove `.repopilot/cache` |
| `repopilot review [path]` | `r` | Review findings that touch changed Git diff lines |
| `repopilot ai context <path>` | — | Generate LLM-ready repository context |
| `repopilot ai plan <path>` | — | Generate a prioritized remediation plan |
| `repopilot ai prompt <path>` | — | Generate a paste-ready remediation prompt |
| `repopilot compare <before> <after>` | `cmp` | Compare two JSON scan reports |
| `repopilot baseline create <path>` | `bl` | Store current findings as accepted debt |
| `repopilot doctor [path]` | `d` | Diagnose audit readiness |
| `repopilot init` | — | Generate `repopilot.toml` |

Get command help:

```bash
repopilot --help
repopilot scan --help
repopilot review --help
repopilot ai --help
```

---

## Output Formats

```bash
repopilot scan . --format console
repopilot scan . --format json --output report.json
repopilot scan . --format markdown --output report.md
repopilot scan . --format html --output report.html
repopilot scan . --format sarif --output repopilot.sarif
```

Use:

- `console` for local development
- `markdown` for human-readable reports
- `json` for scripts and integrations
- `html` for shareable reports
- `sarif` for GitHub Code Scanning

---

## Configuration

Generate a config file:

```bash
repopilot init
```

Run with an explicit config:

```bash
repopilot scan . --config repopilot.toml
```

Configuration precedence:

```text
CLI args > repopilot.toml > built-in defaults
```

Common scan options:

```bash
repopilot scan . --preset strict
repopilot scan . --preset balanced
repopilot scan . --preset lenient
repopilot scan . --max-file-loc 500
repopilot scan . --max-directory-depth 6
repopilot scan . --exclude generated
repopilot scan . --include-low-signal
```

See [configuration docs](docs/configuration.md).

---

## Baseline and Review

Create a baseline for existing findings:

```bash
repopilot baseline create .
```

Scan against accepted debt:

```bash
repopilot scan . --baseline .repopilot/baseline.json
```

Fail CI only on new high-risk findings:

```bash
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
```

Review a pull request or branch diff:

```bash
repopilot review . --base origin/main --fail-on-priority p1
```

See:

- [CLI reference](docs/cli.md)
- [Commands guide](docs/commands.md)
- [GitHub Code Scanning integration](docs/integrations/github-code-scanning.md)

---

## What RepoPilot Checks

RepoPilot includes rules for:

- architecture risk
- large and complex files
- deep nesting
- risky barrel files
- excessive fan-out and coupling
- circular dependencies
- TODO/FIXME/HACK markers
- long functions
- hardcoded secret candidates
- committed private keys
- committed `.env` files
- missing test structure
- source files without test counterparts
- JavaScript/TypeScript framework signals
- React and React Native project health
- Django project health
- Rust panic/unwrap risk
- JavaScript runtime exit risk

See the full rule list in [rulesets](docs/rulesets.md).

---

## Example

```bash
$ repopilot scan .

RepoPilot Scan
Version: 0.12.0
Path: .
Risk: Elevated
Health score: 93/100
Profile: default
Findings visible: 7
Hidden suggestions: 657 strict-only suggestions

Top risks:
	P0 security.secret-candidate src/config/app.ts:12
	P1 language.rust.panic-risk src/api/routes/search.rs:88

Hidden suggestions breakdown:
	 379 testing-gap / testing.source-without-test
	 104 maintainability / code-quality.long-function
		43 maintainability / architecture.large-file
```

Default mode shows what is likely to matter now. Strict mode keeps the full audit:

```bash
repopilot scan . --profile strict
```

---

## Documentation

| Document | Description |
|---|---|
| [Install](docs/install.md) | Cargo, npm, Homebrew, curl, and source builds |
| [AI workflows](docs/ai-workflows.md) | Claude Code, ChatGPT, Cursor, and remediation workflows |
| [Security](docs/security.md) | Local-first trust model and vulnerability reporting |
| [Configuration](docs/configuration.md) | `repopilot.toml`, presets, ignore files, and baseline adoption |
| [Language support](docs/language-support.md) | Supported language/framework tiers and limitations |
| [Knowledge Engine](docs/knowledge-engine.md) | Rule applicability and local-first learning policy |
| [Risk Engine](docs/risk-engine.md) | Risk scoring, priorities, and calibration policy |
| [Trust Mode](docs/trust-mode.md) | Default vs strict visibility and hidden suggestions |
| [Core Visibility Engine](docs/core-visibility-engine.md) | Finding intent model and visibility decisions |
| [CLI reference](docs/cli.md) | Commands, flags, and exit codes |
| [Commands guide](docs/commands.md) | Task-oriented command examples |
| [Rulesets](docs/rulesets.md) | Implemented rules, categories, and severity levels |
| [React Native](docs/react-native.md) | React Native and Expo detection |
| [GitHub Code Scanning](docs/integrations/github-code-scanning.md) | SARIF workflow and CI setup |
| [Roadmap](docs/roadmap.md) | Pre-1.0 roadmap and v1 gates |
| [Release process](docs/release.md) | Manual release process |
| [Changelog](CHANGELOG.md) | Version history |

---

## Roadmap

Planned direction:

- stronger Trust Mode calibration
- `repopilot eval` with golden fixtures
- local feedback through `.repopilot/feedback.yml`
- hidden suggestion trend tracking
- deeper dependency graph and impact analysis
- AI task packs with context, constraints, and acceptance criteria

See [roadmap](docs/roadmap.md).

---

## License

RepoPilot is licensed under **MIT OR Apache-2.0**.
