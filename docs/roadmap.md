# RepoPilot Roadmap

RepoPilot is a review-first, local CLI for maintainers and coding agents. The
product should help answer: what changed, which boundaries moved, and how far
the change reaches before merge.

## Now: 0.24 — Calibrated Trust

RepoPilot 0.24 makes the 0.23 Change Proof mean what it says:

- **calibrated priority** — a finding's priority is capped by its severity, so a
  `P0`/`P1` gate fails for the reasons a maintainer expects;
- **explained proof** — every `limited` result and unmet obligation names its
  cause and one fix, grouped so large changes stay readable;
- **deeper semantic contracts** — typed public API changes for TypeScript,
  JavaScript, and Rust, and contract families validated on real repositories;
- **measured quality** — more rules with labeled real-repository evidence, each
  stating its reviewer model and statistical power;
- **a release that publishes itself** — every channel through Trusted
  Publishing, verified by digest, with a rehearsal path before the final tag.

Details: [v0.24 roadmap and release contract](roadmap/v0.24.md).

## Shipped: 0.23 — Change Proof

One canonical Change Proof per review — a single verdict with reasons, typed
dependency, delivery, runtime-configuration, and security/test contract
changes, proof obligations satisfied only by explicitly selected checks,
optional intent drift and critical paths, and one next action across console,
Markdown, JSON, SARIF, the HTML Change Map, MCP, and the GitHub Action. Details:
[v0.23 release contract](roadmap/v0.23.md).

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
