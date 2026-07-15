# RepoPilot Roadmap

RepoPilot is a review-first, local CLI for maintainers and coding agents. The
product should help answer: what changed, which boundaries moved, and how far
the change reaches before merge.

## Now: 0.21 — agent-run review as the default workflow

The analysis engine is ahead of its adoption. This cycle is about making the
core promise — deterministic review of a change you didn't write — trivially
easy to wire in, and letting real users drive what gets built next.

- documented guardrail recipes: session snapshot + stop-hook review for
  Claude Code, MCP bootstrap for agent clients, review-first CI gate
  (see [Guard your agent runs](agent-guardrail.md));
- a reproducible real-repo demo (pinned zoo checkout) showing a plausible
  agent edit caught by `repopilot review`;
- README and docs lead with change review; scan/baseline positioned as the
  adoption surface;
- precision work only in response to reported noise — the zoo regression
  gate holds the current signal quality; no new speculative rules.

No hosted service, telemetry, source upload, or implicit LLM integration is
part of the roadmap.

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
