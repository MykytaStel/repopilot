# Common Workflows

This guide covers the main RepoPilot workflows. Use the [CLI reference](cli.md)
for every command and flag.

## Review Before Merge

Review local staged, unstaged, and untracked changes:

```bash
repopilot review .
```

Review a branch range:

```bash
repopilot review . --base origin/main
repopilot review . --base origin/main --head HEAD
```

Write machine-readable JSON and SARIF from the same review:

```bash
repopilot review . \
  --base origin/main \
  --format json \
  --output review.json \
  --sarif-output review.sarif
```

Finding gates and review-signal gates are independent:

```bash
repopilot review . --baseline .repopilot/baseline.json --fail-on new-high
repopilot review . --fail-on-priority p1
repopilot review . --fail-on-review definitely
```

Use `--scope full --profile strict` only when a change review also needs the
complete repository audit.

## Review An Agent Run

Create a marker before an agent starts editing:

```bash
repopilot snapshot
```

Review every commit and working-tree change made after the marker:

```bash
repopilot review --since-snapshot
```

## Adopt The Full Scan

Check the repository and confirm the initial setup:

```bash
repopilot init
repopilot scan .
```

Adopt existing debt as a reviewed baseline:

```bash
repopilot baseline create .
repopilot scan . \
  --baseline .repopilot/baseline.json \
  --fail-on new-high
```

Do not refresh the baseline only to make CI pass. A baseline update is an
explicit acceptance of current findings.

Default scans prioritize high-trust findings. Use strict mode for cleanup and
rule calibration:

```bash
repopilot scan . --profile strict
```

## Reports

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot scan . --format json --output repopilot-report.json
repopilot scan . --format sarif --output repopilot.sarif
repopilot scan . --format html --output repopilot.html
```

Add a compact receipt when CI or a release needs provenance:

```bash
repopilot scan . \
  --format markdown \
  --output repopilot-report.md \
  --receipt .repopilot/receipt.json
```

See [Reports](reports.md) for schema and compatibility details.

## GitHub Pull Requests

Use the reusable workflow:

```yaml
jobs:
  repopilot:
    uses: MykytaStel/repopilot/.github/workflows/repopilot-pr-review.yml@v0.19.0
    with:
      fail-on-review: none
      upload-sarif: false
```

Or use the Action directly:

```yaml
- uses: MykytaStel/repopilot@v0.19.0
  with:
    command: review
    scope: changed
```

See [GitHub pull request integration](integrations/github-code-scanning.md) for
permissions, artifacts, SARIF, and sticky comments.

## AI Context

RepoPilot formats local evidence into one assistant-ready handoff without calling
an AI service — context, evidence, a prioritized P0–P3 plan, edit order, working
rules, and verification in a single document:

```bash
repopilot ai context . --budget 4k
repopilot ai context . --focus security --output ai-context.md
repopilot ai context . --no-task | pbcopy
```

For an agent-assisted change, mark the starting state, prepare focused context,
and review the complete result:

```text
snapshot -> context/plan -> change -> review
```

Keep remediation prompts scoped to one risk category or priority group. Require
the agent to preserve unrelated behavior, add focused tests, and report any
verification it could not run.

For false-positive or noise-reduction work, ask the agent to keep strict-mode
recall intact: downgrade or hide low-confidence/default noise rather than deleting
signals, and require a false-negative guard test. Generic dedupe should only merge
exact duplicate emissions (`rule_id` + file + line + snippet + compatible
metadata); broader aggregation belongs in the specific audit that can prove
several locations are one logical issue.

For direct agent integration, run the local MCP server:

```bash
repopilot mcp --root .
```

## Changed scans

Changed scans use `.repopilot/cache/` and skip repository-wide audits:

```bash
repopilot scan . --changed
repopilot scan . --since origin/main
repopilot cache clear .
```

Use a normal full scan when repository-wide architecture and framework findings
must be authoritative.
