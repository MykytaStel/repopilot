# Common Workflows

This guide covers the main RepoPilot workflows. Use the [CLI reference](cli.md)
for every command and flag.

## Choose A Workflow

| Situation | Start here | Why |
|---|---|---|
| I changed code locally | `repopilot review .` | Focuses on the current Git diff |
| I am reviewing a branch or pull request | `repopilot review . --base origin/main` | Compares the branch against its merge base |
| I want a before/after review | `repopilot snapshot`, then `repopilot review --since-snapshot` | Uses Git state; it does not establish authorship |
| I want a complete repository audit | `repopilot scan .` | Includes repository-wide rules |
| I am adopting RepoPilot in an older repository | `repopilot baseline create .` | Separates accepted debt from new findings |
| I need evidence for an external assistant | `repopilot ai context .` | Produces a bounded local handoff |
| I want an agent to call RepoPilot directly | `repopilot mcp --root .` | Exposes local MCP tools; review can run configured checks only when explicitly selected |

By default, MCP review analyzes and returns evidence without running configured
checks. An agent can request a check only by supplying its configured ID through
`repopilot_review_change.verify`; those checks run with host permissions. See
the [MCP tool contract](mcp.md#tool-contract) and [security model](security.md#explicit-verification-commands).

Do not treat changed-scope analysis as a full repository audit. Do not regenerate a
baseline merely to make a gate pass: a baseline update records an explicit acceptance
of current findings.

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

### Read the review result

Start with `Decision` for the human action, then use `Change Proof` to inspect
the evidence and limits behind it. The labels answer related but distinct
questions:

| Field | What it answers |
|---|---|
| `Decision` | What should happen next: `PASS`, `REVIEW`, `BLOCK`, or `NOT ASSESSED`. It maps from the Change Proof verdict; without configured verification, a change with nothing flagged is `PASS (not verified)`. |
| `Change Proof` | What the analyzed scope and selected proof policy establish: `VERIFIED`, `REVIEW`, `BROKEN`, or `NOT ASSESSED`. `VERIFIED` requires a sufficient policy and no outstanding proof reasons; `BROKEN` means supported evidence shows a changed contract is broken. |
| CI and review gates | Did the configured finding threshold or review-signal threshold pass? They are shown separately. A failed configured gate also appears as a proof reason and can make the decision `REVIEW`. |
| `merge_readiness` | A compatibility record with the older `ready`, `review`, and `blocked` values. It is not interchangeable with Change Proof. |

The mapping is `VERIFIED` → `PASS`, `REVIEW` → `REVIEW`, `BROKEN` → `BLOCK`,
and `NOT ASSESSED` → `NOT ASSESSED`.

One exception covers repositories without verification setup. When no
verification check is configured or recorded and the only open proof reasons
are that missing setup (no sufficient proof policy, required checks
unavailable), the decision is `PASS (not verified)`: RepoPilot flagged nothing,
and no check verified the change. Change Proof stays `REVIEW`. Any review
signal, finding, coverage limit, limited contract, failed gate, or intent drift
keeps the decision at `REVIEW`. Once a repository configures a check, the
mapping above applies without the exception.

For example, a review can report `merge_readiness: ready` and Change Proof
`REVIEW` when no sufficient proof policy is selected. The legacy record does not
include that proof-policy reason. Do not treat `ready` as equivalent to
`VERIFIED`.

Use [`--fail-on` or `--fail-on-priority`](cli.md#gates) to set a finding
threshold, and `--fail-on-review` to enable the review-signal gate. The
[configuration walkthrough](configuration.md#first-review-choose-and-run-a-check)
shows how to select a repository verification check.

## Compare Risk Across Runs

History is local, bounded, and disabled by default. Record two compatible
analyses to see what is new, persisting, resolved, or changed severity:

```bash
repopilot scan . --record-history
repopilot scan . --record-history --format json
```

The same opt-in is available for review:

```bash
repopilot review . --record-history
```

Receipts live under `.repopilot/history/` and are never uploaded. RepoPilot
compares only runs with the same target, scope, revisions, profile, filters,
configuration, overlay, and report schema; it never treats an incompatible
changed-only run as proof that full-scan findings were resolved. For the full
workflow — reading the delta, pairing it with baseline resolved-tracking, and
wiring a periodic health scan — see
[Track repository health over time](repository-health.md).

## Review Work Since a Marker

Create a marker before a person or coding agent starts editing:

```bash
repopilot snapshot
```

Review changes since the recorded Git starting point:

```bash
repopilot review --since-snapshot
```

The marker stores `HEAD` and whether the working tree was `dirty`. When it was,
the snapshot also records a baseline commit holding those uncommitted files,
including untracked ones that are not ignored, and pins it as
`refs/repopilot/snapshot`; the index, working tree, and branches are untouched.
The review then diffs from the baseline, so pre-existing changes stay out.
Snapshots written before 0.24 have no baseline and can include pre-existing
changes. Either way, `--since-snapshot` shows what changed since the marker; it
cannot establish which actor authored each change. See the
[snapshot reference](cli.md#snapshot).

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

## Bootstrap Integrations

Generate a review-first GitHub Actions workflow:

```bash
repopilot init --github-action
```

Generate an MCP configuration example without editing the external client:

```bash
repopilot init --mcp-client claude
repopilot init --mcp-client cursor
repopilot init --mcp-client generic
```

Generate the default config plus both integration bootstraps:

```bash
repopilot init --all
```

Generated files are deterministic and are not overwritten unless `--force` is passed.

## GitHub Pull Requests

Use the reusable workflow:

```yaml
jobs:
  repopilot:
    uses: MykytaStel/repopilot/.github/workflows/repopilot-pr-review.yml@v0.24.3
    with:
      fail-on-review: none
      upload-sarif: false
```

Or use the Action directly:

```yaml
- uses: MykytaStel/repopilot@v0.24.3
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
