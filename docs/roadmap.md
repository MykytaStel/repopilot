# RepoPilot Roadmap

RepoPilot is a local review tool for understanding Git changes before merge.
The roadmap records current product outcomes, released milestones, and the
quality gates that guide each release.

## Current: 0.24 — release prepared

RepoPilot 0.24 reports the checks a change weakened and runs the same review
at the end of a coding-agent session. The stable release is prepared; the
`v0.24.0` tag and public-channel verification are still pending. The latest
stable release is 0.23.0.

See the [0.24 release contract](roadmap/v0.24.md) and
[prepared release notes](releases/v0.24.0.md).

## Next: 0.25

The 0.25 scope is not set yet. Evidence from 0.24 points at these candidates:

- failures a change silences in code, such as an empty `catch` or a fallback
  that returns its input, which the agent eval found where test-integrity
  signals saw nothing;
- fewer `test removed` reports when a test is rewritten for a deliberate
  behavior change;
- trivialized tests, loosened matchers, cross-file assertion helpers, and
  bulk-suppression files;
- measuring Claude Code alongside Codex in the agent eval.

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
