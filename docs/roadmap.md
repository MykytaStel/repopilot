# RepoPilot Roadmap

RepoPilot is a review-first, local CLI for maintainers and coding agents. The
product should help answer: what changed, which boundaries moved, and how far
the change reaches before merge.

## Now: 0.22 — repository intelligence and local verification

RepoPilot 0.22 turns the existing `review` and `scan` workflows into two
projections of one evidence-driven repository intelligence core:

- **fast change review** for developers and coding agents, using the Git diff
  plus only the repository context needed to explain risk and blast radius;
- **incremental repository health** for maintainers, covering architecture
  drift, broken-code evidence, antipatterns, hotspots, and risk trends;
- **explicit local verification** through repository-configured, allowlisted
  checks whose results participate in the same canonical readiness decision;
- **measured signal quality and performance**, with zoo-backed rule evidence,
  deterministic cache behavior, and separate review/scan budgets.

The release deepens the existing commands and MCP tools. It does not restore
removed diagnostic commands, add a hosted service, upload source, introduce
telemetry, or put an implicit LLM inside RepoPilot. Details:
[v0.22 roadmap and release contract](roadmap/v0.22.md).

## Shipped: 0.21

Compatible local risk history, ownership-aware merge readiness, repository
knowledge overlays, unified language frontend contracts, and deeper
field-sensitive taint-lite precision. Details:
[v0.21 release contract](roadmap/v0.21.md).

## Shipped: 0.20

Parse-once analysis sessions with a content-addressed cache, unified review
deltas (boundary, behavior, algorithm, taint-lite) with dependency impact
paths, a canonical decision record across CLI/JSON/SARIF/MCP/Action surfaces,
MCP analysis handles with pagination, verdict-first CLI output, and the
real-repo zoo promoted to release evidence. Details:
[v0.20 roadmap and release contract](roadmap/v0.20.md).

## Later

- finalize deprecations and compatibility policy before `1.0`;
- consider curated knowledge packs only after existing signal quality remains
  healthy;
- define the smallest stable `1.0` command and schema contract.

## Release Gates

Every release must keep:

- local-only runtime behavior;
- deterministic findings and review signals;
- fixture-backed stable rules;
- transparent suppressions and hidden suggestions;
- clean self-scan and rule-quality gates;
- compatible CLI, JSON, SARIF, baseline, receipt, Action, and MCP surfaces;
- verified official distribution channels.

The goal is a trustworthy product contract, not the largest rule catalog.
