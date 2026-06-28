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
| [`review`](#review) | `r` | Review findings that touch changed Git diff lines |
| [`scan`](#scan) | `s` | Scan a project, folder, or file for findings |
| [`ai context`](#ai-context) | — | Generate an LLM-ready handoff (context, plan, and guidance) from a scan |
| [`cache`](#cache) | — | Manage local changed-scan cache files |
| [`baseline`](#baseline) | `bl` | Manage accepted baseline findings |
| [`baseline create`](#baseline-create) | — | Scan a path and store current findings as accepted debt |
| [`init`](#init) | — | Generate a default `repopilot.toml` configuration file |
| [`mcp`](#mcp) | — | Run a local Model Context Protocol server over stdio |

---

## `scan`

Walks the target path and runs all enabled audit rules.

**Categories:**

| Category | What it checks |
|----------|---------------|
| Architecture | Oversized files, deep nesting, deep relative imports, risky barrel files, too many modules per directory |
| Coupling | Excessive fan-out, high-instability hubs, circular dependencies |
| Code quality | Cyclomatic complexity, long functions and deep control-flow nesting (AST-based for Rust, TypeScript, JavaScript, Python, Go, Java, C#, Kotlin), TODO/FIXME/HACK markers |
| Framework | JavaScript, React, React Native, Expo, New Architecture, Hermes, Codegen; Python (Django, Flask, FastAPI); Go (Gin, Echo, Fiber) |
| Security | Hardcoded secret candidates, committed private keys, `.env` files, Django deployment settings and raw SQL formatting |
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
| `--output-style` | `compact\|full` | `compact` | Console output style; use `full` for detailed diagnostics |
| `--quiet` | flag | — | Suppress progress indicators and next-step hints while keeping findings and status |
| `--no-progress` | flag | — | Disable progress indicators |
| `--max-findings` | `N\|none` | compact: 5, full/Markdown: all | Limit rendered human-format finding details; JSON and SARIF remain complete |
| `--color` | `auto\|always\|never` | `auto` | Console color mode |
| `--no-color` | flag | — | Disable ANSI color in console output |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |
| `--receipt` | path | — | Write a compact audit receipt JSON file with tool, git, scope, finding, language, and health metadata |
| `--config` | path | auto-detected | Path to a `repopilot.toml` config file |
| `--baseline` | path | — | Path to a baseline file; marks findings as new or existing |
| `--fail-on` | threshold | — | Finding gate by severity/status; exit code 1 on a breach (see [Gates](#gates)) |
| `--fail-on-priority` | `p0\|p1\|p2\|p3` | — | Finding gate by risk priority; mutually exclusive with `--fail-on` |
| `--ignore-feedback` | flag | — | Ignore `.repopilot/feedback.yml` local suppressions |
| `--max-file-loc` | integer | `300` | Maximum non-empty LOC before a file is flagged as large |
| `--max-directory-modules` | integer | `20` | Maximum files per directory before flagging |
| `--max-directory-depth` | integer | `5` | Maximum nesting depth before flagging |
| `--exclude` | path/name | — | Exclude an exact path relative to the scan root or a file/directory name; repeatable |
| `--include-low-signal` | flag | — | Analyze test, fixture, example, generated, and benchmark paths that are skipped by default |
| `--max-file-size` | size | `2097152` | Skip files larger than this size; accepts bytes, `kb`, `mb`, or `gb`; `0` disables the guard |
| `--max-files` | integer | — | Analyze at most this many discovered files after ignore and exclude filters |
| `-w, --workspace` | flag | — | Scan each detected workspace package separately and group findings by package |
| `--changed` | flag | — | Scan only files changed against `HEAD`, including untracked files; repo-level rules are skipped |
| `--since` | git ref | — | Scan only files changed between `BASE...HEAD`; repo-level rules are skipped |
| `--min-severity` | `info\|low\|medium\|high\|critical` | — | Only show findings at or above this severity |
| `--min-confidence` | `low\|medium\|high` | — | Only show findings at or above this confidence |
| `--min-priority` | `p0\|p1\|p2\|p3` | — | Only show findings at or above this risk priority |
| `--verbose` | flag | — | Print scan/render timing after the report |
| `--profile` | `default\|strict` | `default` | Default hides low-signal suggestions; strict shows all findings |
| `--include-maintainability` | flag | — | Include maintainability and testing suggestions hidden by the default profile |
| `--timing` | flag | — | Print pipeline timing for discovery, file analysis, framework detection, audits, enrichment, risk scoring, and report finalization |
| `--preset` | `strict\|balanced\|lenient` | — | Apply a threshold preset without editing config |
| `--rule` | rule ID | — | Only show findings for specific rule IDs; repeatable |

`files_discovered` in JSON output means files found after gitignore, `.repopilotignore`, built-in ignores, and `--exclude` filters. `files_analyzed` means analyzed text files; skipped large files, low-signal paths, binary/unreadable files, and files beyond `--max-files` are not included. JSON also includes `files_skipped_low_signal` and `binary_files_skipped`.

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success (no gate breach) |
| `1` | The finding gate (`--fail-on` / `--fail-on-priority`) was breached |
| `2` | Invalid CLI/config/user input |
| `3` | Runtime or environment failure |

### Examples

```bash
# Basic scan
repopilot scan .
repopilot scan src/
repopilot scan . --output-style full
repopilot scan . --quiet
repopilot scan . --no-progress
repopilot scan . --max-findings 20
repopilot scan . --max-findings none
repopilot scan . --no-color

# Save report to a file
repopilot scan . --format json --output report.json
repopilot scan . --format markdown --output report.md
repopilot scan . --format html --output report.html
repopilot scan . --format sarif --output repopilot.sarif
repopilot scan . --format markdown --output repopilot-report.md --receipt .repopilot/receipt.json

# Use a custom config
repopilot scan . --config repopilot.toml

# Baseline-aware scan
repopilot scan . --baseline .repopilot/baseline.json

# Fail CI on new high or critical findings
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high

# Bypass local feedback suppressions
repopilot scan . --ignore-feedback

# Override thresholds at the command line
repopilot scan . --max-file-loc 500 --max-directory-modules 30 --max-directory-depth 8

# Limit scan input
repopilot scan . --exclude generated --exclude fixtures
repopilot scan . --max-file-size 1mb --max-files 1000
repopilot scan . --include-low-signal

# Monorepo scan with less noise
repopilot scan . --workspace --min-severity medium

# Focus on changed files
repopilot scan . --changed
repopilot scan . --since main

# One-shot threshold presets and timing
repopilot scan . --preset strict
repopilot scan . --profile strict
repopilot scan . --verbose
```

Changed scans write local cache files under `.repopilot/cache/` and intentionally
skip repo-level architecture, framework, testing, and coupling rules. Use a full
scan for authoritative repository-wide risk. Changed-scan summaries include
`cache_telemetry` with cache hits, misses, skipped files, changed-file reasons,
per-file cache decisions, and cache timing impact.

---

## `cache`

Manage RepoPilot's local scan cache.

### Synopsis

```bash
repopilot cache clear [PATH]
```

`cache clear` removes only `.repopilot/cache` for the selected path and succeeds
when the cache directory does not exist.

---

## `review`

Reviews changed files with the default finding profile and repository graph
context. `--scope changed` is the default and omits out-of-diff findings.
Use `--scope full --profile strict` for the previous full-repository audit view.

When coupling data is available, review also shows **blast radius**: files that import changed files and may need extra attention.

By default, review compares the working tree against `HEAD` (staged, unstaged, and untracked changes). Pass `--base` to review a branch range for CI.

Review has two independent gate axes (see [Gates](#gates)): the **finding gate**
(`--fail-on` by severity or `--fail-on-priority` by risk, scoped to in-diff
findings) and the **review-signal gate** (`--fail-on-review definitely`, an
opt-in gate for unsuppressed, gate-eligible definitely-sensitive review signals).
They compose: either one failing exits non-zero.

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
| `--since-snapshot` | flag | — | Review committed and uncommitted work since the last `repopilot snapshot` |
| `--scope` | `changed\|full` | `changed` | Analyze changed files or retain full-repository findings |
| `--profile` | `default\|strict` | scope-dependent | Finding visibility; changed defaults to default, full defaults to strict |
| `--format` | `console\|json\|markdown` | `console` | Output format |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |
| `--sarif-output` | path | — | Write secondary review SARIF without a second scan |
| `--config` | path | auto-detected | Path to a `repopilot.toml` config file |
| `--baseline` | path | — | Path to a baseline file |
| `--fail-on` | threshold | — | Finding gate by severity/status on **in-diff** findings (see [Gates](#gates)) |
| `--fail-on-priority` | `p0\|p1\|p2\|p3` | — | Finding gate by risk priority on **in-diff** findings; mutually exclusive with `--fail-on` |
| `--fail-on-review` | `none\|definitely` | `none` | Review-signal gate; config peer `[review] fail_on` |
| `--no-progress` | flag | — | Disable progress indicators |
| `--ignore-feedback` | flag | — | Ignore `.repopilot/feedback.yml` local suppressions |
| `--max-file-loc` | integer | `300` | Maximum non-empty LOC before a file is flagged as large |
| `--max-directory-modules` | integer | `20` | Maximum files per directory before flagging |
| `--max-directory-depth` | integer | `5` | Maximum nesting depth before flagging |
| `--min-severity` | `info\|low\|medium\|high\|critical` | — | Only show findings at or above this severity |
| `--min-confidence` | `low\|medium\|high` | — | Only show findings at or above this confidence |
| `--min-priority` | `p0\|p1\|p2\|p3` | — | Only show findings at or above this risk priority |

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success (no threshold breach) |
| `1` | A finding gate or explicit review-signal gate failed |
| `2` | Invalid CLI/config/user input |
| `3` | Runtime or environment failure |

### Examples

```bash
# Review uncommitted changes (working tree vs HEAD)
repopilot review .

# Review a branch in CI
repopilot review . --base origin/main
repopilot review . --base origin/main --head HEAD
repopilot review . --base origin/main --no-progress

# Save a Markdown review report
repopilot review . --base origin/main --format markdown --output review.md

# Preserve the previous full-repository strict review
repopilot review . --scope full --profile strict

# Emit machine-readable JSON and SARIF from one review
repopilot review . --format json --output review.json --sarif-output review.sarif

# Baseline-aware CI gate on in-diff findings only
repopilot review . --baseline .repopilot/baseline.json --fail-on new-high
repopilot review . --base origin/main --fail-on-priority p1
repopilot review . --base origin/main --fail-on-review definitely

# Review without local feedback suppressions
repopilot review . --ignore-feedback

# JSON output for downstream tooling
repopilot review . --format json --output review.json

# Focus on high-risk findings only
repopilot review . --min-severity high
```

---

## `ai context`

Scans the repository and formats one LLM-ready handoff as structured Markdown for pasting into Claude Code, Cursor, ChatGPT, or another assistant — or for piping to the clipboard.

The handoff bundles a risk summary, tech stack signals, findings grouped by category with evidence snippets, the Context Risk Graph edit order, a prioritized P0–P3 remediation plan, working rules, a verification checklist, and an approximate token count. The standalone `ai plan` and `ai prompt` commands were folded into this single handoff. Pass `--no-task` to drop the agent guidance (task, rules, and verification) and emit fact-only context — the same form the MCP `context` tool returns.

For false-positive work, remember that this handoff is product-facing context.
The default profile may hide low-confidence suggestions; use `scan`/`review` (or
the MCP tools) with `--profile strict` when you need recall validation before
deciding whether a signal should be downgraded, hidden, or kept visible.

`ai context` emits Markdown by default. Pass `--format json` for a structured,
deterministic handoff — project, risk summary, repository facts, focus-filtered
findings (each with stable id, `risk` score/priority/signals, description,
recommendation, and the full `evidence` list), and the P0–P3 plan — the same facts
without Markdown parsing, matching the JSON the MCP tools return. The JSON form is
**budget-aware**: findings are ordered by risk and added until the output reaches
`--budget`, and the document reports `truncated` plus included/omitted counts, so
the budget is honest. `--no-header`/`--no-task` affect Markdown only (JSON is
always fact-only), and JSON output never mixes in the stderr token breakdown.

### Synopsis

```
repopilot ai context <PATH> [OPTIONS]
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
| `--format` | `markdown\|json` | `markdown` | Output format: human-readable Markdown, or structured JSON for agents |
| `--no-header` | flag | — | Omit the intro header block |
| `--no-task` | flag | — | Omit the AI task instruction preamble |
| `--show-breakdown` | flag | — | Print a per-section token breakdown to stderr (automatic when stdout is a TTY) |

### Examples

```bash
repopilot ai context .
repopilot ai context . --focus security --budget 2k
repopilot ai context . --output ai-context.md
repopilot ai context . --no-task --output ai-context.md
repopilot ai context . --format json --output ai-context.json
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
repopilot baseline create . --output .repopilot/baseline.json

# Custom output path
repopilot baseline create . --output ./baseline.json

# Overwrite existing baseline
repopilot baseline create . --output .repopilot/baseline.json --force
```

Treat `.repopilot/baseline.json` as accepted existing debt. Commit or update it
only after intentional review, and include a PR note explaining why the findings
are accepted. Do not update it just to make CI green.

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

## `mcp`

Runs a local Model Context Protocol (MCP) server over stdio so AI agents can call RepoPilot as a tool. It speaks JSON-RPC 2.0 on stdin/stdout and is launched by the MCP client, not run interactively. Nothing is uploaded and no AI service is called; each tool runs the same local analysis as the CLI.

See [docs/mcp.md](mcp.md) for the tool catalog, schemas, and agent registration.

### Synopsis

```
repopilot mcp [--root PATH]
```

### Tools

| Tool | Description |
|------|-------------|
| `repopilot_review_change` | Changed/full review with findings, tiered signals, blast radius, and gate result |
| `repopilot_scan` | Full repository audit as a JSON report |
| `repopilot_context` | Budgeted, AI-ready Markdown brief (optional `focus`, `budget`) |
| `repopilot_explain_file` | How one file is classified and which rules and signals apply |
| `repopilot_explain_finding` | Replay a file-scoped emitted finding by stable ID from the current MCP session |

### Examples

```bash
# Register with Claude Code
claude mcp add repopilot -- repopilot mcp --root .

# Manual smoke test (list the available tools)
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | repopilot mcp --root .
```

---

## Gates

RepoPilot has two independent gate axes. Each can exit the process with code `1`;
they compose, so a build fails if **either** trips.

| Gate | Flags | Acts on | Available in |
|------|-------|---------|--------------|
| **Finding gate** | `--fail-on` (severity/status) **or** `--fail-on-priority` (risk) | Rule findings | `scan`, `review` |
| **Review-signal gate** | `--fail-on-review` / config `[review] fail_on` | Review signals (behavioral/boundary/taint) | `review` |

- The two finding-gate flags are **mutually exclusive** — pick severity *or*
  priority, not both. On `review` the finding gate evaluates only **in-diff**
  findings; on `scan` it evaluates the whole report.
- The review-signal gate is a **different axis** from the finding gate: despite
  the similar name, the config key `[review] fail_on` (`none`/`definitely`) is the
  peer of `--fail-on-review`, **not** of `--fail-on`. `definitely` fails on
  unsuppressed, gate-eligible definitely-sensitive review signals.

### Finding-gate thresholds

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

`--fail-on-priority` accepts `p0`, `p1`, `p2`, or `p3` and fails on any finding at
or above that risk priority.

The `--min-severity` flag filters rendered findings before gate evaluation, and is not itself a gate. Use it when a local report is too noisy, for example `--min-severity high` during fast review or `--workspace --min-severity medium` in monorepos.

---

## Output formats

| Format | Available in | Best for |
|--------|-------------|----------|
| `console` | `scan`, `review` | Versioned terminal report with risk summary, top risk clusters, and grouped findings |
| `json` | `scan`, `review` | Machine consumption, piping to scripts |
| `markdown` | `scan`, `review` | Versioned human-readable report with top rules and findings index |
| `html` | `scan` | Standalone visual report with severity, category, and rule filters |
| `sarif` | `scan` | GitHub Code Scanning, CI security tooling |

See [docs/integrations/github-code-scanning.md](integrations/github-code-scanning.md) for the SARIF upload workflow.
