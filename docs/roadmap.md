# RepoPilot Roadmap

RepoPilot is a review-first, local CLI for maintainers and coding agents. The
product should help answer: what changed, which boundaries moved, and how far
the change reaches before merge.

## Now: 0.23 — Change Proof

RepoPilot 0.23 turns the evidence-driven review core into one bounded proof of
the change:

- **typed contract changes** explain what changed across public symbols,
  dependencies, delivery workflows, runtime configuration, security boundaries,
  and tests;
- **consumer proof chains** connect each contract delta to resolved direct and
  bounded transitive impact;
- **one canonical ChangeProof** combines evidence, coverage, ownership,
  verification, intent drift, limitations, gate behavior, and the next action;
- **truth and release confidence** close known compatibility, quality,
  documentation, performance, and publication gaps before release claims ship.

The release deepens the existing commands and MCP tools. It does not add a new
top-level command, hosted service, source upload, telemetry, implicit LLM,
autofix, or automatic repository command execution. Details:
[v0.23 roadmap and release contract](roadmap/v0.23.md).

## Shipped: 0.22

Unified graph-backed repository intelligence, conservative broken import/export
evidence, explicit allowlisted local verification, resolved baseline findings,
truthful assessment output, and measured real-repository rule quality. Details:
[v0.22 release contract](roadmap/v0.22.md).

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
