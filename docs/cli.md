# CLI Reference

Complete reference for all RepoPilot commands and flags.

## Synopsis

```
repopilot <COMMAND> [OPTIONS]
```

Use `-h` for a short summary or `--help` for the full description and examples.

---

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| [`scan`](#scan) | `s` | Scan a project, folder, or file for findings |
| [`review`](#review) | `r` | Review findings that touch changed Git diff lines |
| [`vibe`](#vibe) | `v` | Generate LLM-ready context from a scan |
| [`harden`](#harden) | `h` | Generate a prioritized remediation plan |
| [`prompt`](#prompt) | `p` | Generate an AI-ready remediation prompt |
| [`compare`](#compare) | `cmp` | Compare two JSON scan reports and show what changed |
| [`baseline`](#baseline) | `bl` | Manage accepted baseline findings |
| [`baseline create`](#baseline-create) | — | Scan a path and store current findings as accepted debt |
| [`init`](#init) | — | Generate a default `repopilot.toml` configuration file |

---

## `scan`

Walks the target path and runs all enabled audit rules.

**Categories:**

| Category | What it checks |
|----------|---------------|
| Architecture | Oversized files, deep nesting, too many modules per directory |
| Coupling | Excessive fan-out, high-instability hubs, circular dependencies |
| Code quality | Cyclomatic complexity, long functions, TODO/FIXME/HACK markers |
| Framework | JavaScript, React, React Native, Expo, New Architecture, Hermes, Codegen |
| Security | Hardcoded secret candidates, committed private keys, `.env` files |
| Testing | Missing test folder, source files without test counterparts |

The scan respects `.gitignore` and built-in ignores for common build directories.
For React Native and Expo projects, JSON/Markdown/HTML summaries include architecture profile metadata; see [React Native Analysis](react-native.md).

### Synopsis

```
repopilot scan <PATH> [OPTIONS]
repopilot s <PATH> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<PATH>` | Path to project, folder, or file |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--format` | `console\|json\|markdown\|html\|sarif` | `console` | Output format |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |
| `--config` | path | auto-detected | Path to a `repopilot.toml` config file |
| `--baseline` | path | — | Path to a baseline file; marks findings as new or existing |
| `--fail-on` | threshold | — | Exit code 1 when findings meet this threshold (see [Thresholds](#thresholds)) |
| `--max-file-loc` | integer | `300` | Maximum non-empty LOC before a file is flagged as large |
| `--max-directory-modules` | integer | `20` | Maximum files per directory before flagging |
| `--max-directory-depth` | integer | `5` | Maximum nesting depth before flagging |
| `-w, --workspace` | flag | — | Scan each detected workspace package separately and group findings by package |
| `--min-severity` | `info\|low\|medium\|high\|critical` | — | Only show findings at or above this severity |
| `--verbose` | flag | — | Print scan phase timing breakdown after the report |
| `--preset` | `strict\|balanced\|lenient` | `balanced` | Apply a threshold preset without editing config |

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success (no threshold breach) |
| `1` | Findings exceed the `--fail-on` threshold, or a runtime error occurred |

### Examples

```bash
# Basic scan
repopilot scan .
repopilot scan src/

# Save report to a file
repopilot scan . --format json --output report.json
repopilot scan . --format markdown --output report.md
repopilot scan . --format html --output report.html
repopilot scan . --format sarif --output repopilot.sarif

# Use a custom config
repopilot scan . --config repopilot.toml

# Baseline-aware scan
repopilot scan . --baseline .repopilot/baseline.json

# Fail CI on new high or critical findings
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high

# Override thresholds at the command line
repopilot scan . --max-file-loc 500 --max-directory-modules 30 --max-directory-depth 8

# Monorepo scan with less noise
repopilot scan . --workspace --min-severity medium

# One-shot threshold presets and timing
repopilot scan . --preset strict
repopilot scan . --verbose
```

---

## `review`

Scans the repository and separates findings into two groups:

- **in-diff** — findings on lines that appear in the current Git diff
- **out-of-diff** — findings elsewhere in the codebase

When coupling data is available, review also shows **blast radius**: files that import changed files and may need extra attention.

By default, review compares the working tree against `HEAD` (staged, unstaged, and untracked changes). Pass `--base` to review a branch range for CI.

When `--fail-on` is used, the gate evaluates **only in-diff findings** so unrelated pre-existing issues do not block the pipeline.

### Synopsis

```
repopilot review [PATH] [OPTIONS]
repopilot r [PATH] [OPTIONS]
```

### Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `[PATH]` | `.` | Path to project, folder, or file |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--base` | git ref | — | Base ref for the diff; without this, compares working tree to `HEAD` |
| `--head` | git ref | `HEAD` | Head ref; requires `--base` |
| `--format` | `console\|json\|markdown` | `console` | Output format |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |
| `--config` | path | auto-detected | Path to a `repopilot.toml` config file |
| `--baseline` | path | — | Path to a baseline file |
| `--fail-on` | threshold | — | Exit code 1 when **in-diff** findings meet this threshold |
| `--max-file-loc` | integer | `300` | Maximum non-empty LOC before a file is flagged as large |
| `--max-directory-modules` | integer | `20` | Maximum files per directory before flagging |
| `--max-directory-depth` | integer | `5` | Maximum nesting depth before flagging |
| `--min-severity` | `info\|low\|medium\|high\|critical` | — | Only show findings at or above this severity |

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success (no threshold breach) |
| `1` | In-diff findings exceed the `--fail-on` threshold, or a runtime error occurred |

### Examples

```bash
# Review uncommitted changes (working tree vs HEAD)
repopilot review .

# Review a branch in CI
repopilot review . --base origin/main
repopilot review . --base origin/main --head HEAD

# Save a Markdown review report
repopilot review . --base origin/main --format markdown --output review.md

# Baseline-aware CI gate on in-diff findings only
repopilot review . --baseline .repopilot/baseline.json --fail-on new-high

# JSON output for downstream tooling
repopilot review . --format json --output review.json

# Focus on high-risk findings only
repopilot review . --min-severity high
```

---

## `vibe`

Scans the repository and formats findings as structured Markdown for pasting into Claude Code, Cursor, ChatGPT, or another LLM assistant.

The output includes a risk summary, tech stack signals, findings grouped by category, evidence snippets, fix recommendations, and an approximate token count.
`vibe` emits Markdown only; it does not accept `--format` and does not change JSON or SARIF schemas.

### Synopsis

```
repopilot vibe <PATH> [OPTIONS]
repopilot v <PATH> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<PATH>` | Path to project, folder, or file |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--config` | path | auto-detected | Path to a `repopilot.toml` config file |
| `--focus` | `security\|arch\|architecture\|quality\|framework\|all` | `all` | Limit output to a category |
| `--budget` | `2k\|4k\|8k\|16k` or positive integer | `4k` | Target token budget |
| `-o, --output` | path | stdout | Write output to a file instead of stdout |
| `--no-header` | flag | — | Omit the intro header block |

### Examples

```bash
repopilot vibe .
repopilot vibe . --focus security --budget 2k
repopilot vibe . --output vibe.md
repopilot vibe . --no-header | pbcopy
```

---

## `harden`

Scans the repository and formats findings as a Markdown hardening plan with P0/P1/P2/P3 priorities, locations, rule IDs, fix recommendations, and verification commands.

`harden` emits Markdown only; it does not accept `--format` and does not change JSON or SARIF schemas.

### Synopsis

```
repopilot harden <PATH> [OPTIONS]
repopilot h <PATH> [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--config` | path | auto-detected | Path to a `repopilot.toml` config file |
| `--focus` | `security\|arch\|architecture\|quality\|framework\|all` | `all` | Limit output to a category |
| `--budget` | `2k\|4k\|8k\|16k` or positive integer | `4k` | Target token budget |
| `-o, --output` | path | stdout | Write output to a file instead of stdout |

### Examples

```bash
repopilot harden .
repopilot harden . --focus security --budget 2k
repopilot harden . --output harden.md
```

---

## `prompt`

Scans the repository and emits a Markdown prompt for a coding assistant, including remediation instructions and embedded RepoPilot context.

`prompt` emits Markdown only; it does not call an AI service, accept `--format`, or change JSON/SARIF schemas.

### Synopsis

```
repopilot prompt <PATH> [OPTIONS]
repopilot p <PATH> [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--config` | path | auto-detected | Path to a `repopilot.toml` config file |
| `--focus` | `security\|arch\|architecture\|quality\|framework\|all` | `all` | Limit embedded context to a category |
| `--budget` | `2k\|4k\|8k\|16k` or positive integer | `4k` | Target token budget for embedded context |
| `-o, --output` | path | stdout | Write output to a file instead of stdout |

### Examples

```bash
repopilot prompt .
repopilot prompt . --focus security --budget 2k
repopilot prompt . --output prompt.md
```

---

## `compare`

Diffs two RepoPilot JSON scan reports and shows which findings are new, resolved, or unchanged.

### Synopsis

```
repopilot compare <BEFORE> <AFTER> [OPTIONS]
repopilot cmp <BEFORE> <AFTER> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<BEFORE>` | Path to the earlier scan report (JSON) |
| `<AFTER>` | Path to the more recent scan report (JSON) |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--format` | `console\|json\|markdown` | `console` | Output format |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |

### Examples

```bash
# Capture before/after and compare
repopilot scan . --format json --output before.json
# ... make your changes ...
repopilot scan . --format json --output after.json
repopilot compare before.json after.json

# Markdown diff report
repopilot compare before.json after.json --format markdown

# JSON diff for scripting
repopilot compare before.json after.json --format json --output diff.json
```

---

## `baseline`

Manages the accepted baseline file. Currently exposes one subcommand: [`baseline create`](#baseline-create).

### Synopsis

```
repopilot baseline <SUBCOMMAND>
repopilot bl <SUBCOMMAND>
```

---

## `baseline create`

Runs a full scan and writes all current findings to a baseline file. Future scans with `--baseline` will mark each matching finding as `existing` and flag only genuinely new findings.

By default writes to `.repopilot/baseline.json` and creates the directory if needed. Existing files are not overwritten unless `--force` is passed.

### Synopsis

```
repopilot baseline create <PATH> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<PATH>` | Path to project, folder, or file |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-o, --output` | path | `.repopilot/baseline.json` | Write baseline to a custom path |
| `--force` | flag | — | Overwrite an existing baseline file |

### Examples

```bash
# Create baseline in the default location
repopilot baseline create .

# Custom output path
repopilot baseline create . --output ./baseline.json

# Overwrite existing baseline
repopilot baseline create . --force
```

---

## `init`

Writes a `repopilot.toml` with all configurable thresholds at their default values. Edit the file to tune thresholds for your project.

Configuration precedence: CLI flags > `repopilot.toml` > built-in defaults.

### Synopsis

```
repopilot init [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--force` | flag | — | Overwrite an existing config file |
| `--path` | path | `repopilot.toml` | Path where the config file should be written |

### Examples

```bash
repopilot init
repopilot init --force
repopilot init --path ./config/repopilot.toml
```

---

## Thresholds

The `--fail-on` flag accepts the following values:

| Value | Meaning |
|-------|---------|
| `new-low` | Fail when any **new** low, medium, high, or critical finding exists |
| `new-medium` | Fail when any **new** medium, high, or critical finding exists |
| `new-high` | Fail when any **new** high or critical finding exists |
| `new-critical` | Fail when any **new** critical finding exists |
| `low` | Fail when any finding of low severity or higher exists |
| `medium` | Fail when any finding of medium severity or higher exists |
| `high` | Fail when any finding of high severity or higher exists |
| `critical` | Fail when any critical finding exists |

`new-*` thresholds require a `--baseline` to distinguish new from existing findings. Without a baseline, all current findings are treated as new.

For `review`, `--fail-on` evaluates only **in-diff** findings.

The `--min-severity` flag filters rendered findings before baseline or CI gate evaluation. Use it when a local report is too noisy, for example `--min-severity high` during fast review or `--workspace --min-severity medium` in monorepos.

---

## Output formats

| Format | Available in | Best for |
|--------|-------------|----------|
| `console` | `scan`, `review`, `compare` | Interactive terminal use |
| `json` | `scan`, `review`, `compare` | Machine consumption, piping to scripts |
| `markdown` | `scan`, `review`, `compare` | Human-readable reports, PR comments |
| `html` | `scan` | Standalone visual reports |
| `sarif` | `scan` | GitHub Code Scanning, CI security tooling |

See [docs/integrations/github-code-scanning.md](integrations/github-code-scanning.md) for the SARIF upload workflow.
