# RepoPilot Roadmap

RepoPilot is a review-first, local CLI for maintainers and coding agents. The
product should help answer: what changed, which boundaries moved, and how far
the change reaches before merge.

## Now: 0.17

- reduce false positives in behavioral, algorithmic, and taint-lite signals;
- keep self-scan and review-quality gates free of accepted local suppressions;
- make release inputs, Actions, tools, and official channel publication
  deterministic;
- keep CLI help, doctor, docs, Action, and MCP aligned around change review;
- preserve JSON/SARIF, Action output, and MCP compatibility while signals remain
  at preview.

No new rule family, distribution channel, hosted service, telemetry, or implicit
LLM integration enters `0.17`.

## Next

- collect adoption evidence through reproducible reports, issues, and user
  feedback;
- improve first-run examples from observed user failures;
- publish a compatibility and deprecation window for the smallest `1.0`
  command/schema surface.

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
