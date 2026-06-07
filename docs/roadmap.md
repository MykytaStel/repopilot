# RepoPilot Roadmap

RepoPilot is a review-first, local CLI for maintainers and coding agents. The
product should help answer: what changed, which boundaries moved, and how far
the change reaches before merge.

## Now: 0.16

- ship changed-scope review as the default;
- stabilize review signal identity, provenance, suppression, and opt-in gates;
- keep GitHub Action, MCP, and preview VSIX consumers on the same JSON/SARIF
  review;
- simplify packaging, documentation, branches, and release automation;
- keep Cargo, npm, Homebrew, and GitHub Releases reliable.

## Next: 0.17

- reduce false positives in boundary, behavioral, algorithmic, and taint-lite
  signals;
- expand true-positive and false-positive fixtures for default-visible review;
- measure adoption through reproducible demos, issues, and user feedback rather
  than new channel count;
- improve first-run docs and CI evidence;
- document compatibility expectations for review JSON, SARIF, Action outputs,
  and MCP schemas.

No new rule family, Marketplace channel, PyPI package, hosted service, telemetry,
or implicit LLM integration should enter `0.17` without demonstrated user
demand and a maintenance owner.

## Later

- finalize deprecations and compatibility policy before `1.0`;
- review whether preview VSIX distribution has enough demand for a supported
  Marketplace channel;
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
