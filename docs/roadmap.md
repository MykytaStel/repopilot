# RepoPilot Roadmap

RepoPilot is a local review tool for understanding Git changes before merge.
The roadmap records current product outcomes, released milestones, and the
quality gates that guide each release.

## Current: 0.24 — Honest Green

RepoPilot 0.24 reports when a change turns CI green by weakening the checks
that judge it:

- skipped or focused tests, removed test cases or assertions, and expected
  values rewritten together with the code they test;
- added suppressions and relaxed CI, coverage, or type-checking gates;
- a labeled corpus of real pull requests that measures how often this happens
  and what RepoPilot catches, with its limits stated;
- optional snapshot, stop-hook, commit-hook, and CI integrations that run the
  same check before review;
- severity-capped priorities, named coverage limits, and grouped proof
  obligations, already on `main`.

See the [0.24 release contract](roadmap/v0.24.md).

## Released

- **0.23 — Change Proof:** one review verdict, its reasons, proof obligations,
  and next action across supported outputs. [Release notes](releases/v0.23.0.md)
- **0.22 — Repository intelligence:** shared graph analysis, broken local
  import/export evidence, explicit local verification, and resolved baselines.
  [Release notes](releases/v0.22.0.md)
- **0.21 — Local risk history:** ownership-aware readiness, repository
  overlays, and deeper field-sensitive taint analysis.
  [Release notes](releases/v0.21.0.md)
- **0.20 — Review workflows:** canonical scan and review decisions, MCP
  analysis, and GitHub Action integration.
  [Release notes](releases/v0.20.0.md)

## Longer-term direction

- Define a stable `1.0` command, configuration, and report contract.
- Consider curated knowledge packs after signal quality remains healthy.
- Expand language and semantic coverage when fixture and repository evidence
  supports the claims.

## Release gates

Every release keeps local analysis, deterministic findings, fixture-backed
rules, visible suppression decisions, clean quality gates, compatible public
schemas, and verified distribution channels. Release notes describe measured
behavior and name meaningful limits.
