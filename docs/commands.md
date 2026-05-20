# Commands Guide

Task-oriented guide to RepoPilot commands. For a complete flag reference see [docs/cli.md](cli.md).

---

## Quick start

```bash
repopilot init          # generate repopilot.toml
repopilot doctor .      # check adoption readiness
repopilot scan .        # scan the current directory
repopilot ai context .  # prepare local AI remediation context
```

For React Native or Expo projects:

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot review . --base origin/main --baseline .repopilot/baseline.json --fail-on new-high
```

---

## Scanning a project

`repopilot scan` is the primary command. It walks the target path, runs all audit rules, and prints a report.
For JavaScript workspaces, the summary includes detected framework projects and React Native architecture metadata when present.
Python projects are scanned for Django, Flask, and FastAPI (detected from `requirements.txt`), including Django deployment settings and raw SQL checks when Django is present. Go projects are scanned for Gin, Echo, and Fiber (detected from `go.mod`). Detected frameworks appear in the tech-stack summary produced by `repopilot ai context`.
The walker respects gitignore, `.repopilotignore`, and built-in ignores for common build, cache, vendor, and platform directories, including `.git`, `target`, `node_modules`, `dist`, `build`, `.next`, `.nuxt`, `.cache`, `coverage`, `vendor`, `Pods`, and `DerivedData`.

```bash
repopilot scan .
repopilot scan src/payments/
repopilot scan src/payments/processor.rs
```

### Scanning changed files

Use changed scans when you want a fast file-level pass for the current diff.
RepoPilot writes local cache files under `.repopilot/cache/` and skips
repo-level architecture, framework, testing, and coupling rules in this mode.
Changed-scan summaries include cache hits, misses, skipped changed files,
changed-file reasons, cache decisions, and cache timing impact.

```bash
repopilot scan . --changed
repopilot scan . --since main
repopilot cache clear .
```

Use a regular full scan when you need authoritative repository-wide risk.

### Scanning workspaces

Use `--workspace` in npm, Yarn, pnpm, or Cargo monorepos to scan each package root separately and group findings by package. Console and Markdown output include a compact workspace risk summary.

```bash
repopilot scan . --workspace
repopilot scan . --workspace --min-severity medium
```

When no workspace packages are detected, RepoPilot falls back to a normal single-project scan and prints a warning.

### Saving reports

Use `--output` to write to a file instead of stdout. The format is inferred from `--format`:

```bash
repopilot scan . --format json --output report.json
repopilot scan . --format markdown --output report.md
repopilot scan . --format html --output report.html
repopilot scan . --format sarif --output repopilot.sarif
repopilot scan . --format markdown --output repopilot-report.md --receipt .repopilot/receipt.json
```

Use `--receipt` when you need compact JSON evidence for CI artifacts or release
records. The receipt includes schema version, RepoPilot version, git state, scan
scope, finding counts, language counts, and health score.

### Overriding thresholds

CLI flags override `repopilot.toml` and built-in defaults:

```bash
repopilot scan . --max-file-loc 500
repopilot scan . --max-directory-modules 30
repopilot scan . --max-directory-depth 8
```

Use presets for one-shot tuning without editing config:

```bash
repopilot scan . --preset strict
repopilot scan . --preset lenient
```

### Limiting scan input

Use `--exclude` for exact relative paths or file/directory names, `--max-file-size` to skip large files, and `--max-files` to cap how many discovered files are analyzed:

```bash
repopilot scan . --exclude generated --exclude fixtures
repopilot scan . --max-file-size 1mb
repopilot scan . --max-files 1000
```

Size values accept raw bytes plus `kb`, `mb`, and `gb` suffixes. By default, low-signal audit paths such as tests, fixtures, examples, generated files, and benchmarks are skipped; pass `--include-low-signal` to analyze them.

JSON reports expose this accounting with `files_discovered`, `files_analyzed` (analyzed text files), `files_skipped_low_signal`, `binary_files_skipped`, `large_files_skipped`, and `skipped_bytes`.

### Filtering by severity, confidence, and priority

Use `--min-severity` and `--min-confidence` to reduce local report noise while keeping the same rules enabled:

```bash
repopilot scan . --min-severity high
repopilot review . --min-severity high
repopilot scan . --min-confidence high
repopilot review . --base origin/main --min-confidence medium
```

Use `--min-priority` when you want risk-ranked output instead of severity-only
filtering, and `--rule` when investigating one detector:

```bash
repopilot scan . --min-priority p2
repopilot review . --base origin/main --min-priority p1
repopilot scan . --rule language.rust.panic-risk --timing
```

Use `--verbose` when you need scan and render timing:

```bash
repopilot scan . --verbose
```

Use `--timing` when you need the engine pipeline breakdown. It reports
discovery, file analysis, framework detection, project audits, enrichment, risk
scoring, and report finalization separately.

---

## AI workflows

`repopilot ai context` scans the project and emits structured Markdown with risk summary, grouped findings, evidence snippets, recommendations, and an approximate token count.

```bash
repopilot ai context .
repopilot ai context . --focus security --budget 2k
repopilot ai context . --output vibe.md
repopilot ai context . --no-header | pbcopy
```

Use `--focus security`, `--focus arch`, `--focus quality`, or `--focus framework` to narrow the context before pasting it into Claude Code, Cursor, ChatGPT, or another LLM assistant.
The removed 0.x `repopilot vibe` alias is no longer part of the executable command surface. The GitHub Action can run `command: ai-context`; it defaults the path to `.` and does not pass `--format` because AI commands are Markdown-only.

### Hardening plan

Use `ai plan` when you want a deterministic remediation plan before editing code. It groups findings into P0/P1/P2/P3 priorities, clusters repeated rule patterns by repository area, and includes verification commands.

```bash
repopilot ai plan .
repopilot ai plan . --focus security --budget 2k
repopilot ai plan . --output harden.md
```

### AI-ready prompt

Use `ai prompt` when you want one paste-ready instruction block for a coding assistant. It includes remediation constraints and embedded RepoPilot context.

```bash
repopilot ai prompt .
repopilot ai prompt . --focus security --budget 2k
repopilot ai prompt . --output prompt.md
```

The GitHub Action can also run `command: ai-plan` and `command: ai-prompt`; both commands are Markdown-only.

---

## Command surface policy

Before v1, new user-facing behavior should fit the existing command families:

- New detectors and checks become rules under `scan`.
- New AI-ready outputs become `ai` subcommands or flags.
- New debugging and rule-author tools become `inspect` subcommands.

Top-level commands should stay focused on stable workflows: audit, review, baseline, compare, AI assistance, inspection, initialization, and readiness diagnostics.

---

## Reviewing changes

`repopilot review` scans the full repository but separates findings into **in-diff** (on changed lines) and **out-of-diff** groups. This makes it easier to focus on what the current change introduces.

### Local review (working tree vs HEAD)

```bash
repopilot review .
```

Covers staged, unstaged, and untracked files.

### Branch review (CI)

```bash
repopilot review . --base origin/main
repopilot review . --base origin/main --head HEAD
repopilot review . --base origin/main --fail-on-priority p1
```

When `--fail-on` is used with `review`, only **in-diff findings** trigger a failure — unrelated pre-existing issues do not block CI.
`--fail-on-priority` works the same way, but evaluates P0/P1/P2/P3 risk priority instead of severity.

### Blast radius

When coupling data is available, `review` also lists files that **import** the changed files. These files may be affected by the change and worth an extra look.

---

## Comparing two scans

`repopilot compare` diffs two JSON scan reports and shows what changed between them:

```bash
repopilot scan . --format json --output before.json
# make your changes
repopilot scan . --format json --output after.json
repopilot compare before.json after.json
```

Useful for understanding the impact of a refactor or a dependency update without needing a Git diff.

---

## Baseline workflow

A baseline lets you adopt RepoPilot in a repository that already has findings, without failing CI on pre-existing issues.

### Step 1 — create the baseline

```bash
repopilot baseline create .
```

This scans the project and writes all current findings to `.repopilot/baseline.json`. Commit the file.

### Step 2 — scan with the baseline

```bash
repopilot scan . --baseline .repopilot/baseline.json
```

Findings present in the baseline are marked `existing`. Findings not in the baseline are marked `new`.

### Step 3 — gate CI on new findings only

```bash
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
```

The pipeline fails only when a **new** high or critical finding appears. Pre-existing issues do not block the build.

### Refreshing the baseline

Refresh only when the team explicitly accepts the current findings as technical debt:

```bash
repopilot baseline create . --force
```

Do not refresh blindly to silence CI — that defeats the purpose.

---

## CI integration

Minimal CI step that gates on new high findings:

```yaml
- name: Install RepoPilot
  run: cargo install repopilot

- name: Scan
  run: repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
```

For SARIF upload to GitHub Code Scanning:

```yaml
- name: Scan (SARIF)
  run: repopilot scan . --format sarif --output repopilot.sarif

- name: Upload to GitHub Code Scanning
  uses: github/codeql-action/upload-sarif@v4
  if: always()
  with:
    sarif_file: repopilot.sarif
```

See [docs/integrations/github-code-scanning.md](integrations/github-code-scanning.md) for the full workflow with required permissions.

---

## Generating a config file

```bash
repopilot init
```

Writes `repopilot.toml` with all thresholds at their defaults. Edit the file and commit it. RepoPilot reads it automatically on each `scan`.

```bash
repopilot init --force            # overwrite an existing config
repopilot init --path ./cfg/repopilot.toml
```

---

## Common patterns

### Fail only on critical findings in CI

```bash
repopilot scan . --fail-on critical
```

### Save a Markdown report as a CI artifact

```bash
repopilot review . --base origin/main --format markdown --output review.md
```

### Scan a monorepo without low-severity noise

```bash
repopilot scan . --workspace --min-severity medium
```

### Compare before and after a large refactor

```bash
git stash
repopilot scan . --format json --output before.json
git stash pop
repopilot scan . --format json --output after.json
repopilot compare before.json after.json --format markdown
```

### Scan a single file

```bash
repopilot scan src/payments/processor.rs
```

---

## `inspect cache`

Inspect local changed-scan cache diagnostics.

```bash
repopilot inspect cache .
repopilot inspect cache . --format json
repopilot inspect cache . --format markdown --output cache.md
```

The command is read-only and reports cache entry counts, schema version,
approximate cache size, and stale entry count.

---

## `inspect feedback`

Validate local feedback suppressions and show which ones were applied.

```bash
repopilot inspect feedback .
repopilot inspect feedback . --format json
repopilot inspect feedback . --format markdown --output feedback.md
```

The command parses `.repopilot/feedback.yml` with a YAML parser, reports
malformed entries, warns about suppressions that do not match current findings,
and includes the same `local_feedback` summary that scan/review reports expose.

---

## Local feedback suppressions

RepoPilot reads `.repopilot/feedback.yml` by default:

```yaml
suppressions:
  - rule_id: architecture.large-file
    path: "src/generated/schema.rs"
    reason: generated schema boundary
```

Use raw output without local suppressions:

```bash
repopilot scan . --ignore-feedback
repopilot review . --ignore-feedback
```

When suppressions are applied, console, Markdown, JSON, and receipt output show
the local feedback counts so findings are visibly suppressed rather than
silently disappearing.
