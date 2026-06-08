# Engine pipeline contract

RepoPilot product commands should build one scan summary and pass that summary
through filters, visibility, report schema conversion, and renderers.

## Pipeline

```text
discovery
-> file analysis
-> repository context
-> context graph
-> finding enrichment
-> risk scoring
-> contract validation
-> visibility/filtering
-> report schema
-> renderer
```

## Rules

- Renderers must never trigger a repository scan.
- AI output commands should consume the same product scan path as normal scans.
- Review should not scan once for review and again for report metadata.
- Context-graph output must be built from an existing scan summary, never a re-scan.
- Changed scans must be honest about scope and must not pretend to include all
  full-repository rules.
- Expensive graph/risk summaries should be computed once per command path.

## Why this matters

RepoPilot should behave like a local-first review engine, not a set of separate
commands that each re-walk or re-score the repository differently.
