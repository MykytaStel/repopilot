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
| [`ai context`](#ai-context) | — | Generate LLM-ready context from a scan |
| [`ai plan`](#ai-plan) | — | Generate a prioritized remediation plan |
| [`ai prompt`](#ai-prompt) | — | Generate an AI-ready remediation prompt |
| [`inspect explain`](#inspect-explain) | — | Explain file classification and rule decisions |
| [`inspect knowledge`](#inspect-knowledge) | — | Inspect bundled Knowledge Engine data |
| [`inspect cache`](#inspect-cache) | — | Inspect local changed-scan cache diagnostics |
| [`inspect graph`](#inspect-graph) | — | Inspect Context Risk Graph decisions |
| [`inspect feedback`](#inspect-feedback) | — | Validate local feedback suppressions |
| [`inspect rules`](#inspect-rules) | — | List registered rules with lifecycle and signal metadata |
| [`inspect rule`](#inspect-rule) | — | Inspect one registered rule |
| [`inspect eval-rules`](#inspect-eval-rules) | — | Evaluate registered rules against bundled fixtures |
| [`compare`](#compare) | `cmp` | Compare two JSON scan reports and show what changed |
| [`cache`](#cache) | — | Manage local changed-scan cache files |
| [`baseline`](#baseline) | `bl` | Manage accepted baseline findings |
| [`baseline create`](#baseline-create) | — | Scan a path and store current findings as accepted debt |
| [`doctor`](#doctor) | `d` | Diagnose audit readiness |
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
| `--fail-on` | threshold | — | Exit code 1 when findings meet this threshold (see [Thresholds](#thresholds)) |
| `--fail-on-priority` | `p0\|p1\|p2\|p3` | — | Exit code 1 when findings meet this risk priority threshold |
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
| `0` | Success (no threshold breach) |
| `1` | Findings exceed the `--fail-on` threshold |
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

# Inspect or bypass local feedback suppressions
repopilot inspect feedback .
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
| `--fail-on-priority` | `p0\|p1\|p2\|p3` | — | Exit code 1 when **in-diff** findings meet this risk priority threshold |
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
| `1` | In-diff findings exceed the `--fail-on` threshold |
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

# Baseline-aware CI gate on in-diff findings only
repopilot review . --baseline .repopilot/baseline.json --fail-on new-high
repopilot review . --base origin/main --fail-on-priority p1

# Review without local feedback suppressions
repopilot review . --ignore-feedback

# JSON output for downstream tooling
repopilot review . --format json --output review.json

# Focus on high-risk findings only
repopilot review . --min-severity high
```

---

## `ai context`

Scans the repository and formats findings as structured Markdown for pasting into Claude Code, Cursor, ChatGPT, or another LLM assistant.

The output includes a risk summary, tech stack signals, findings grouped by category, evidence snippets, finding recommendations, and an approximate token count.
`ai context` emits Markdown only and does not accept `--format`.

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
| `--no-header` | flag | — | Omit the intro header block |

### Examples

```bash
repopilot ai context .
repopilot ai context . --focus security --budget 2k
repopilot ai context . --output ai-context.md
repopilot ai context . --no-header | pbcopy
```

---

## `ai plan`

Scans the repository and formats findings as a Markdown AI plan with P0/P1/P2/P3 priorities, locations, rule IDs, finding recommendations, and verification commands.

`ai plan` emits Markdown only and does not accept `--format`.

### Synopsis

```
repopilot ai plan <PATH> [OPTIONS]
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
repopilot ai plan .
repopilot ai plan . --focus security --budget 2k
repopilot ai plan . --output ai-plan.md
```

---

## `ai prompt`

Scans the repository and emits a Markdown prompt for a coding assistant, including remediation instructions and embedded RepoPilot context.

`ai prompt` emits Markdown only; it does not call an AI service or accept `--format`.

The removed 0.x `repopilot prompt` alias is no longer part of the executable command surface; use `repopilot ai prompt`.

### Synopsis

```
repopilot ai prompt <PATH> [OPTIONS]
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
repopilot ai prompt .
repopilot ai prompt . --focus security --budget 2k
repopilot ai prompt . --output prompt.md
```

---

## `inspect explain`

Explains how RepoPilot classifies a single file before applying audits. This is an advanced diagnostic command for rule authors, false-positive debugging, and context-model development.

The removed 0.x `repopilot explain` alias is no longer part of the executable command surface; use `repopilot inspect explain`.

### Synopsis

```
repopilot inspect explain <PATH> [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--rule` | rule ID | — | Evaluate a rule against the file context |
| `--signal` | signal | — | Optional rule signal, for example `rust.unwrap` |
| `--severity` | severity | `medium` | Base severity before Knowledge Engine overrides |
| `--format` | `console\|json\|markdown` | `console` | Output format |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |

### Examples

```bash
repopilot inspect explain src/main.rs
repopilot inspect explain src/main.rs --rule language.rust.panic-risk --signal rust.unwrap
repopilot inspect explain src/App.tsx --format markdown --output explain.md
```

---

## `inspect knowledge`

Prints the bundled Knowledge Engine catalog: languages, frameworks, runtimes, paradigms, and rule applicability records. This is an advanced diagnostic command rather than a normal audit workflow.

The removed 0.x `repopilot knowledge` alias is no longer part of the executable command surface; use `repopilot inspect knowledge`.

### Synopsis

```
repopilot inspect knowledge [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--section` | `all\|languages\|frameworks\|runtimes\|paradigms\|rules` | `all` | Catalog section to render |
| `--format` | `console\|json\|markdown` | `console` | Output format |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |

### Examples

```bash
repopilot inspect knowledge
repopilot inspect knowledge --section languages
repopilot inspect knowledge --section rules --format json
```

---

## `inspect feedback`

Validates `.repopilot/feedback.yml` and reports malformed suppression entries.
By default it only parses the feedback file and renders diagnostics. Use
`--evaluate` to run a repository scan and report matched or unmatched
suppressions against current findings. This command is local-only and does not
upload source code.

### Synopsis

```
repopilot inspect feedback [PATH] [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--format` | `console\|json\|markdown` | `console` | Output format |
| `--evaluate` | flag | — | Scan the repository and evaluate suppressions against current findings |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |

### Examples

```bash
repopilot inspect feedback .
repopilot inspect feedback . --format json
repopilot inspect feedback . --evaluate --format json
repopilot inspect feedback . --format markdown --output feedback.md
```

---

## `inspect cache`

Reports local changed-scan cache path, cache existence, entry counts, and cache metadata. It is read-only.

### Synopsis

```
repopilot inspect cache [PATH] [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--format` | `console\|json\|markdown` | `console` | Output format |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |

### Examples

```bash
repopilot inspect cache .
repopilot inspect cache . --format json
repopilot inspect cache . --format markdown --output cache.md
```

---

## `inspect graph`

Builds a local scan summary and renders Context Risk Graph diagnostics without starting a second scan during rendering.

### Synopsis

```
repopilot inspect graph [PATH] [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--config` | path | auto-detected | Path to a `repopilot.toml` config file |
| `--format` | `console\|json\|markdown` | `console` | Output format |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |

### Examples

```bash
repopilot inspect graph .
repopilot inspect graph . --format json
repopilot inspect graph . --format markdown --output graph.md
```

---

## `inspect rules`

Lists registered rules with lifecycle, signal source, default visibility, fixture coverage, and stability metadata.

### Synopsis

```
repopilot inspect rules [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--format` | `console\|json\|markdown` | `console` | Output format |
| `--lifecycle` | `experimental\|preview\|stable\|deprecated` | — | Filter by lifecycle |
| `--source` | signal source | — | Filter by signal source, for example `text-heuristic` |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |

### Examples

```bash
repopilot inspect rules
repopilot inspect rules --format json
repopilot inspect rules --lifecycle preview
repopilot inspect rules --source ast
repopilot inspect rules --source text-heuristic
```

### Signal sources: AST-backed vs heuristic

Each rule reports a `signal_source`. The authoritative, always-current list is the
tool itself — `repopilot inspect rules --source ast` and
`repopilot inspect rules --source text-heuristic`. As of now:

| Rule | Detection | Notes |
|------|-----------|-------|
| `code-quality.long-function` | AST | Function spans from the syntax tree for Rust, TypeScript/JavaScript, Python, Go, Java, C#, Kotlin; line/brace heuristic only on parse failure. |
| `code-quality.deep-control-flow` | AST | Same language set as long-function. |
| `language.go.panic-exit-risk` | AST | `panic`/`log.Fatal`/`os.Exit` calls from the syntax tree. |
| `language.python.exception-risk` | AST | Bare `except`, `assert`, and `NotImplementedError` from the syntax tree. |
| `language.javascript.runtime-exit-risk` | AST | `process.exit` calls and library-boundary `throw new Error(...)`. |
| `language.managed.fatal-exception-risk` | AST | Generic fatal `throw`s and `TODO()`/not-implemented placeholders in Java/Kotlin/C# from the syntax tree. |
| `language.rust.panic-risk` | AST | `unwrap`/`expect`/`unwrap_err`/`expect_err` calls and `panic!`/`todo!`/`unimplemented!` macros from the syntax tree; severity context stays heuristic. |
| `code-quality.complex-file`, `code-marker.*`, `architecture.*`, `testing.*` | Heuristic | Structural/text signals by design. |

AST-backed rules ignore risky tokens that appear only in comments or string
literals; heuristic rules sanitize comments and strings line-by-line but cannot
reason about full call structure.

---

## `inspect rule`

Shows metadata for one registered rule.

### Synopsis

```
repopilot inspect rule <RULE_ID> [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--format` | `console\|json\|markdown` | `console` | Output format |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |

### Examples

```bash
repopilot inspect rule security.secret-candidate
repopilot inspect rule architecture.circular-dependency --format json
```

---

## `inspect eval-rules`

Runs local fixture projects for registered rules and reports missing, unexpected, contract, stability, and fixture-quality failures.

### Synopsis

```
repopilot inspect eval-rules [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--rule` | rule ID | — | Evaluate only one rule |
| `--fixtures` | path | `tests/fixtures/rules` | Fixture root |
| `--format` | `console\|json\|markdown` | `console` | Output format |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |

### Examples

```bash
repopilot inspect eval-rules
repopilot inspect eval-rules --rule security.secret-candidate
repopilot inspect eval-rules --format json
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

## `doctor`

Runs an audit-readiness check for a repository. It reports scan scope accounting, checks for config, `.repopilotignore`, baseline, Git, generic CI, RepoPilot-specific CI gates, and report/receipt output readiness, then recommends the next adoption command to run.

### Synopsis

```
repopilot doctor [PATH] [OPTIONS]
repopilot d [PATH] [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--config` | path | auto-detected | Path to a `repopilot.toml` config file |
| `--format` | `console\|json\|markdown` | `console` | Output format |
| `-o, --output` | path | stdout | Write report to a file instead of stdout |
| `--include-low-signal` | flag | — | Analyze low-signal paths skipped by default |
| `--max-files` | integer | — | Analyze at most this many discovered files |

### Examples

```bash
repopilot doctor .
repopilot doctor . --format json
repopilot doctor . --format markdown --output doctor.md
```

Doctor keeps its JSON shape additive: new readiness checks appear as extra
`checks[]` entries such as `config_readable`, `baseline_readable`,
`repopilot_ci`, and `report_receipt`.

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
repopilot mcp
```

### Tools

| Tool | Description |
|------|-------------|
| `repopilot_review_change` | Audit the current Git changes: in-diff vs out-of-diff findings plus blast radius |
| `repopilot_scan` | Full repository audit as a JSON report |
| `repopilot_context` | Budgeted, AI-ready Markdown brief (optional `focus`, `budget`) |
| `repopilot_explain_file` | How one file is classified and which rules and signals apply |

### Examples

```bash
# Register with Claude Code
claude mcp add repopilot -- repopilot mcp

# Manual smoke test (list the available tools)
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | repopilot mcp
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
| `console` | `scan`, `review`, `compare` | Versioned terminal report with risk summary, top risk clusters, and grouped findings |
| `json` | `scan`, `review`, `compare` | Machine consumption, piping to scripts |
| `markdown` | `scan`, `review`, `compare` | Versioned human-readable report with top rules and findings index |
| `html` | `scan` | Standalone visual report with severity, category, and rule filters |
| `sarif` | `scan` | GitHub Code Scanning, CI security tooling |

See [docs/integrations/github-code-scanning.md](integrations/github-code-scanning.md) for the SARIF upload workflow.
