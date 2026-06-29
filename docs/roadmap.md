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

## Next: 0.20

- use reviewed zoo dispositions to calibrate the remaining default-visible noise;
- broaden replay/explanation coverage only where the required repository context
  can be reconstructed deterministically;
- keep deprecations explicit and evidence-backed before removing compatibility
  surfaces;
- improve first-run examples from observed user and CI failures.

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
