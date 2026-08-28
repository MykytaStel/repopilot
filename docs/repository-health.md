# Track Repository Health Over Time

Change review answers "is this diff safe to merge." This page is for the other
question: is the repository, as a whole, getting better or worse across
releases — and which findings from last quarter are still open. Everything
here is local; nothing is uploaded.

## Turn on local history

History is opt-in and disabled by default:

```toml
# repopilot.toml
[history]
enabled = true
max_runs = 50
max_bytes = 5242880
```

Or record one run without changing config:

```bash
repopilot scan . --record-history
```

Each run appends a bounded receipt to `.repopilot/history/runs.jsonl` — never
uploaded, ignored by Git and by analysis fingerprints. The same opt-in works
for `review . --record-history`.

## Read the compatible delta

From the second recorded run on, `--format json` carries a `risk_delta`
comparing this run against the most recent *compatible* one — same target,
scope, revisions, profile, filters, config, overlay, and report schema.
RepoPilot never treats an incompatible run (a changed-only scan compared
against a full scan, say) as proof that anything was resolved:

```bash
repopilot scan . --record-history --format json | jq '.risk_delta'
```

```json
{
  "comparison": { "prior_revision": "…", "current_revision": "…" },
  "new_findings": [],
  "persisting_findings": [
    { "rule_id": "security.secret-candidate", "path": "src/app.py", "severity": "HIGH" }
  ],
  "resolved_findings": [],
  "severity_shifts": []
}
```

`new_findings` is what landed since the last compatible run; `resolved_findings`
is what disappeared; `severity_shifts` is a finding whose severity changed
without disappearing. `persisting_findings` is everything unchanged — the
number to watch trend downward. Console and Markdown output do not render this
yet; read it from JSON or the `repopilot_scan`/`repopilot_review_change` MCP
tools.

## Track accepted debt separately from new risk

History answers "what changed since last time." A baseline answers "what did
we already know about and accept":

```bash
repopilot baseline create . --profile strict --min-severity high
```

Scope `--profile`/`--min-severity`/`--min-priority` to match how you'll scan
later — a finding one side's filter hides and the other's does not reads as
spuriously new or resolved, not as unchanged. Later runs classify every
finding against that snapshot:

```bash
repopilot scan . --baseline .repopilot/baseline.json
```

```
New findings: 0
Existing findings: 12
Resolved findings: 3
```

`resolved` — the same idea as history's `resolved_findings`, but against a
snapshot you deliberately accepted rather than the last run — is what proves
debt actually got paid down, not just displaced. In JSON, resolved findings
carry only what the baseline snapshot itself stored (`key`, `rule_id`,
`severity`, `path`, `message`), not a full finding — the source location that
produced one may no longer exist to re-derive it from. Refresh the baseline
only when the team explicitly re-accepts the current state, never as a way to
silence CI: `repopilot baseline create . --force`.

## Wire it into CI as a trend, not a gate

A per-PR gate belongs to [change review](agent-guardrail.md), not here. For a
periodic health signal, schedule a workflow that scans `main` and commits (or
uploads as an artifact) the JSON report:

```yaml
on:
  schedule:
    - cron: "0 6 * * 1"  # weekly
jobs:
  health:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: MykytaStel/repopilot@v0.22.0
        with:
          command: scan
          format: json
```

Diff successive artifacts, or record history in the same job and read
`risk_delta`, to see whether the repository trended toward or away from health
since the last run.

## Know how much to trust a given rule

Not every rule carries the same evidence. The generated
[rule scorecard](engineering/rule-scorecard.md) reports, per rule, whether it
has been measured against real repositories at all, and if so, its precision
estimate and outstanding false-positive debt. A rule with `no zoo evidence` is
unmeasured, which is not the same as clean — weight its findings accordingly
until it earns evidence.
