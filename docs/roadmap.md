# RepoPilot Roadmap

RepoPilot is a local review tool for understanding Git changes before merge.
The roadmap records current product outcomes, released milestones, and the
quality gates that guide each release.

## Current: 0.24 — released

RepoPilot 0.24 reports the checks a change weakened and runs the same review
at the end of a coding-agent session. The stable `v0.24.0` tag is published;
GitHub Releases, crates.io, all six npm packages, Homebrew, and MCP Registry
passed their publication checks.

See the [0.24 release contract](roadmap/v0.24.md),
[release notes](releases/v0.24.0.md), and
[publication evidence](engineering/v0.24-release-scorecard.md#publication).

The `v0.24.1` patch is published on GitHub Releases, crates.io, npm, and
Homebrew: agent stop hooks stop only for weakened checks, once per session; a
change with nothing flagged and no configured verification reads
`PASS (not verified)`; two integrity false positives are fixed. See the
[0.24.1 release notes](releases/v0.24.1.md).

The `v0.24.2` patch is published on GitHub Releases, crates.io, npm, and
Homebrew: plain first-run errors instead of raw Git output, an empty-change
hint that names `--base origin/main`, MCP tool descriptions that route between
sibling tools, and license files that GitHub and MCP directories detect. See
the [0.24.2 release notes](releases/v0.24.2.md).

The `v0.24.3` patch is published on GitHub Releases, crates.io, npm, and
Homebrew: reviews on large repositories are about four times faster (next.js:
~32 s to ~8.5 s) with identical findings, cached and uncached reviews agree,
generated-code and entry-point detection read only headers and top-level
definitions, and two removal false alarms are gone.
See the [0.24.3 release notes](releases/v0.24.3.md).

## Next: 0.25

The next slice develops the analysis engine alongside reliable first use.
Engineering does not wait for the external-user sample. The
[0.25 scope](roadmap/v0.25.md) defines the technical slices and their
acceptance criteria; the [0.25 evidence ledger](engineering/v0.25-evidence-ledger.md)
records progress. The current published version remains 0.24.3.

### Technical core

1. **Resolver freshness and semantic cache parity.** Reproduce the suspected
   stale TypeScript alias cache in a long-lived MCP process. Cover config
   creation, edits, and deletion; compare complete semantic evidence across
   cold and warm runs of the same scope.
2. **Direct JavaScript/TypeScript default-export contracts.** Extend the
   existing named-export check to a removed default export whose precisely
   resolved relative caller still imports it. Preserve coordinated changes
   and uncertain resolution as safe or explicitly limited cases.
3. **Structural Rust public-item contracts.** Start with a removed public
   function and a proven direct local caller. Signature and field changes,
   macros, traits, conditional compilation, and re-exports need separate slices.
4. **One measured taint blind spot.** Begin with C# SQL assigned to a command
   object's property and executed on that same object. Cover parameterization,
   clean overwrites, unrelated objects, and no execution before widening scope.
5. **Large-graph performance protection.** Measure many-file/import workloads
   before adding resolver indexes. Require deterministic evidence and unchanged
   ambiguity handling as well as lower time and memory costs.

The initial engineering scope is the first two slices. The remaining three are
approved technical slices, each with focused tests and review evidence. Every slice
needs an unsafe regression, safe controls, applicable full/changed/cache/MCP
parity, and evidence appropriate to its claims.

### First use and evidence

Progress alongside the technical work:

1. **Installation and first review.** Validate a fresh public CLI install and
   agent setup in five minutes for Claude Code, Codex, Gemini CLI, and Cursor;
   validate Copilot's setup and PR review separately. Record actual client and
   platform versions. Target five independently observed first reviews; the
   owner's scripted tests are compatibility evidence, not that user sample.
2. **Fewer misleading interruptions.** Review real reports of rewritten or
   renamed tests. Add a false-positive regression and a recall guard before
   adjusting a signal. Measure the affected corpus cases, not a headline total
   across unrelated rules.
3. **One missed behavior, measured first.** The Codex eval found code changed
   to satisfy contradictory tests while integrity signals stayed quiet. Collect
   concrete error-swallowing/fallback cases, define benign guards, and evaluate
   an advisory signal before adding an automatic stop condition.

Additional candidates from 0.24 evidence:

- trivialized tests, loosened matchers, cross-file assertion helpers, and
  bulk-suppression files;
- measuring Claude Code alongside Codex in the agent eval.

Track successful first reviews, user-confirmed useful reports, false alarms,
and recurring installation failures. Downloads, stars, and code mentions are
distribution counters; they do not establish active use or detector quality.
Support and release maintenance follow the
[agent integration runbook](engineering/maintaining-agent-integrations.md).

## Released

- **0.24.3 — Large-repository reviews:** about four times faster on next.js
  with identical findings; cached and uncached reviews agree; structural
  generated-code and entry-point detection. [Release notes](releases/v0.24.3.md)
- **0.24.2 — Plain first-run errors:** review explains a missing repository,
  commit, ref, or merge base; MCP tools say when to use each one.
  [Release notes](releases/v0.24.2.md)
- **0.24.1 — Quieter agent stops:** stop hooks only for weakened checks, an
  informative first decision, two integrity false positives fixed.
  [Release notes](releases/v0.24.1.md)
- **0.24 — Honest Green:** weakened checks, session review, and verified agent
  installation channels. [Release notes](releases/v0.24.0.md)
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
