# RepoPilot Roadmap

RepoPilot is a review-first, local CLI for maintainers and coding agents. The
product should help answer: what changed, which boundaries moved, and how far
the change reaches before merge.

## Now: 0.21 — trust the default output, calibrate it to your repo

One larger release, three layers. The language frontend contract landed
first (registry, per-language tables, generated support matrix, contributor
guide); the rest of the cycle builds on it:

- **contract honesty to zero** — complete C# (imports, taint, boundary),
  account for Rust's dedicated panic-risk audit, Kotlin taint, framework
  probes on frontends; the support ledger ends empty;
- **local knowledge overlays** — a declarative, diffable local file that
  calibrates known rules to a repository (severity, suppression) without
  code execution or plugins;
- **local analysis history** — a `.repopilot/` ledger of past runs:
  risk deltas vs the previous scan and accumulated agent-session evidence;
- **new languages via the cheap path** — added as frontend modules with a
  pinned zoo repository as the acceptance gate;
- guardrail recipes, reproducible agent-edit demos, and the review-first
  README continue as the adoption surface.

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
