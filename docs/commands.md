# Commands Guide

Task-oriented guide to RepoPilot commands. For a complete flag reference see [docs/cli.md](cli.md).

---

## Quick start

```bash
repopilot init        # generate repopilot.toml
repopilot scan .      # scan the current directory
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
Python projects are scanned for Django, Flask, and FastAPI (detected from `requirements.txt`); Go projects for Gin, Echo, and Fiber (detected from `go.mod`). Detected frameworks appear in the tech-stack summary produced by `repopilot vibe`.

```bash
repopilot scan .
repopilot scan src/payments/
repopilot scan src/payments/processor.rs
```

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
```

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

### Filtering by severity

Use `--min-severity` to reduce local report noise while keeping the same rules enabled:

```bash
repopilot scan . --min-severity high
repopilot review . --min-severity high
```

Use `--verbose` when you need scan and render timing:

```bash
repopilot scan . --verbose
```

---

## Vibe Check for LLM workflows

`repopilot vibe` scans the project and emits structured Markdown with risk summary, grouped findings, evidence snippets, recommendations, and an approximate token count.

```bash
repopilot vibe .
repopilot vibe . --focus security --budget 2k
repopilot vibe . --output vibe.md
repopilot vibe . --no-header | pbcopy
```

Use `--focus security`, `--focus arch`, `--focus quality`, or `--focus framework` to narrow the context before pasting it into Claude Code, Cursor, ChatGPT, or another LLM assistant.
The GitHub Action can run `command: vibe`; it defaults the path to `.` and does not pass `--format` because `vibe` is Markdown-only.

### Hardening plan

Use `harden` when you want a deterministic remediation plan before editing code. It groups findings into P0/P1/P2/P3 priorities and includes verification commands.

```bash
repopilot harden .
repopilot harden . --focus security --budget 2k
repopilot harden . --output harden.md
```

### AI-ready prompt

Use `prompt` when you want one paste-ready instruction block for a coding assistant. It includes remediation constraints and embedded RepoPilot context.

```bash
repopilot prompt .
repopilot prompt . --focus security --budget 2k
repopilot prompt . --output prompt.md
```

The GitHub Action can also run `command: harden` and `command: prompt`; both commands are Markdown-only.

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
```

When `--fail-on` is used with `review`, only **in-diff findings** trigger a failure — unrelated pre-existing issues do not block CI.

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
