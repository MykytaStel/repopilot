# RepoPilot Roadmap

RepoPilot is a review-first, local CLI for maintainers and coding agents. The
product should help answer: what changed, which boundaries moved, and how far
the change reaches before merge.

## Now: 0.18 — evidence you can click

- point architecture findings at the real import line instead of `line 1`, so a
  cycle or boundary violation lands on the code that caused it;
- model detected workspaces (npm/yarn, pnpm, Cargo, `go.work`) as first-class
  `Package` nodes, and auto-enable `architecture.package-boundary-violation` on
  them without configuration;
- score complexity per function (`code-quality.complex-function`, preview) by
  nesting depth rather than counting branches flatly across a whole file;
- grow the trust surface: a before/after review golden harness, fixtures for the
  previously unpinned heuristic rules, a registry-generated
  [rules reference](rules-reference.md), and config discovery that walks up to
  the git root;
- preserve JSON/SARIF, Action output, baseline, and MCP compatibility while new
  signals stay at preview.

No new rule family, distribution channel, hosted service, telemetry, or implicit
LLM integration enters `0.18`. The JSON schema stays at `0.19`.

## Next: 0.19

- demote `code-quality.complex-file` now that the per-function rule covers the
  honest cases (removal lands in `0.20`);
- open a deprecation window for explicit `[architecture] package_roots` once
  workspace auto-detection has field data;
- back the remaining unfixtured heuristic rules with true/false-positive
  fixtures and broaden the review golden matrix;
- collect adoption evidence through reproducible reports, issues, and user
  feedback, and improve first-run examples from observed failures.

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
