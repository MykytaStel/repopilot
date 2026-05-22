# RepoPilot

[![Crates.io](https://img.shields.io/crates/v/repopilot.svg)](https://crates.io/crates/repopilot)
[![npm](https://img.shields.io/npm/v/repopilot.svg)](https://www.npmjs.com/package/repopilot)
[![CI](https://github.com/MykytaStel/repopilot/actions/workflows/ci.yaml/badge.svg)](https://github.com/MykytaStel/repopilot/actions)
[![GitHub Release](https://img.shields.io/github/v/release/MykytaStel/repopilot)](https://github.com/MykytaStel/repopilot/releases)
[![License](https://img.shields.io/crates/l/repopilot.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/MykytaStel/repopilot?style=social)](https://github.com/MykytaStel/repopilot)

**Local-first repository risk checks for safer human and AI-assisted code changes.**

RepoPilot gives teams a fast safety pass before a pull request, release, or AI
coding session. It scans a repository locally, explains architecture, security,
code-quality, testing, and framework risks, then turns findings into AI-ready
context without uploading your source code to a hosted scanner.

It is not meant to replace language linters. RepoPilot focuses on repository
signals that are hard to see from one file at a time: risky files, architecture
drift, security candidates, testing gaps, review risk, baselines, and remediation
context for Claude Code, ChatGPT, Cursor, or another coding assistant.

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

Use RepoPilot when you need to:

- understand the highest-risk files before asking an AI assistant to edit;
- add a CI gate without blocking on every existing finding;
- review PR risk from changed lines instead of only whole-repo noise;
- produce local evidence for releases through Markdown, JSON, SARIF, and receipts.

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

First five minutes:

```bash
repopilot scan .
repopilot doctor .
repopilot baseline create . --output .repopilot/baseline.json
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
repopilot review . --base origin/main
repopilot ai context . --budget 4k
```

That path answers what RepoPilot found, whether the repo is ready for CI
adoption, which findings are new, what changed in the current review, and what
local context to paste into an AI coding assistant.

Save audit evidence for CI or release review:

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot scan . --format json --output repopilot-report.json --receipt .repopilot/receipt.json
repopilot scan . --format sarif --output repopilot.sarif
```

Copy AI context on macOS:

```bash
repopilot ai context . | pbcopy
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
| `repopilot inspect rules` | — | Inspect rule lifecycle and signal-source metadata |
| `repopilot inspect eval-rules` | — | Run local rule fixture evaluation |
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
repopilot baseline create . --output .repopilot/baseline.json
```

Scan against accepted debt:

```bash
repopilot scan . --baseline .repopilot/baseline.json
```

Fail CI only on new high-risk findings:

```bash
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
```

Commit `.repopilot/baseline.json` only after reviewing the accepted findings.
Do not update it just to make CI green; note accepted baseline changes in the PR.

Validate or bypass local feedback suppressions:

```bash
repopilot inspect feedback .
repopilot scan . --ignore-feedback
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

## Finding Trust

RepoPilot 0.13.0 is a breaking cleanup release focused on trustable findings,
a stronger local scan engine, rule lifecycle discipline, signal quality metrics,
and pre-v1 product contract hardening.

Every rendered finding is enriched and validated before reporting. JSON reports
include finding provenance, `risk-v3` signals, raw-vs-visible finding counts,
`raw_signal_quality`, and `visible_signal_quality`. The default scan is a normal
actionable scan: high-priority runtime, security, maintainability, and stable
import-graph risks stay visible, while low-signal testing and marker noise stays
available through `--profile strict`. Rule metadata is inspectable locally:

```bash
repopilot inspect rules
repopilot inspect rules --lifecycle preview
repopilot inspect rules --source text-heuristic
repopilot inspect rule security.secret-candidate
repopilot inspect eval-rules --rule security.secret-candidate
```

RepoPilot remains local-first: no telemetry, no source upload, no hosted scanner,
no arbitrary plugin execution, and no LLM API calls.

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
Version: 0.13.0
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
| [Local Feedback](docs/local-feedback.md) | Repository-local suppressions and validation |
| [Trust Mode](docs/trust-mode.md) | Default vs strict visibility and hidden suggestions |
| [Core Visibility Engine](docs/core-visibility-engine.md) | Finding intent model and visibility decisions |
| [CLI reference](docs/cli.md) | Commands, flags, and exit codes |
| [Commands guide](docs/commands.md) | Task-oriented command examples |
| [Rulesets](docs/rulesets.md) | Implemented rules, categories, and severity levels |
| [Rule quality gate](docs/rule-quality-gate.md) | Fixture, metadata, visibility, and stability expectations for rules |
| [React Native](docs/react-native.md) | React Native and Expo detection |
| [GitHub Code Scanning](docs/integrations/github-code-scanning.md) | SARIF workflow and CI setup |
| [Roadmap](docs/roadmap.md) | Pre-1.0 roadmap and v1 gates |
| [0.12 GTM plan](docs/gtm-0.12.md) | Launch audiences, messaging, and proof points |
| [Release process](docs/release.md) | Manual release process |
| [Changelog](CHANGELOG.md) | Version history |

---

## Roadmap

Planned direction:

- stronger Trust Mode calibration
- `repopilot eval` with golden fixtures
- local feedback validation and report transparency
- hidden suggestion trend tracking
- deeper dependency graph and impact analysis
- AI task packs with context, constraints, and acceptance criteria

See [roadmap](docs/roadmap.md).

---

## License

RepoPilot is licensed under **MIT OR Apache-2.0**.
