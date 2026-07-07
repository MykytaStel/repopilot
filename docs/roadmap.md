# RepoPilot Roadmap

RepoPilot is a review-first, local CLI for maintainers and coding agents. The
product should help answer: what changed, which boundaries moved, and how far
the change reaches before merge.

## Now: 0.19 — replayable evidence and lower default noise

- make MCP explanations replayable from emitted findings while preserving stable
  baseline IDs and occurrence-level evidence selection;
- carry Knowledge Engine decision provenance in schema `0.20` reports so agents
  can compare stored and current decisions deterministically;
- expose file-role evidence, executable-package manifest context, and ordered
  decision traces without changing scan/review severity behavior;
- keep strict-mode recall while moving low-confidence false positives out of the
  default profile;
- use the real-repo zoo snapshots and reviewed expectations as release evidence.

No hosted service, telemetry, source upload, or implicit LLM integration is part
of the current roadmap.

## Next: 0.20 — Trusted Change Intelligence

See [roadmap/v0.20.md](roadmap/v0.20.md) for the full release contract,
performance benchmarks, staged PR sequence, and release scorecard.

- immutable analysis session and shared parsed facts for parse-once performance;
- incremental context graph and content-addressed cache v2;
- unified boundary, behavior, algorithm, and taint-lite review deltas with
  dependency impact paths and deterministic verification plans;
- canonical decision record across CLI, JSON, SARIF, MCP, AI context, and
  GitHub Action;
- promoted real-repo zoo as release evidence with review-zoo differential
  fixtures;
- MCP workspace revisions, analysis handles, and pagination;
- verdict-first CLI output with progressive disclosure;
- delta-focused GitHub Action PR comments and stable artifacts;
- hardened analysis boundaries, redaction, and cache corruption recovery.

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
