# RepoPilot 0.23: Proof Card and Replayable Evidence Design

- Status: draft for review
- Date: 2026-09-20
- Base: `origin/main` at `61db9a8f63a50e93790a5d5fc78c9799afb287c1`
- Package version: `0.22.0` during development

## Problem

RepoPilot has the core pieces of the 0.23 direction: ChangeProof, typed
contract deltas, proof obligations, intent and critical paths, projection
parity, sandbox evidence, and opt-in CI policy. They are still difficult to
evaluate as one product because a user must reconstruct the decision from
several outputs and because an emitted result is not yet a single replayable
evidence object with a clear boundary.

The 0.23 development line must improve the product and its proof of quality
before any version bump or release action. It must make the meaningful change
visible to a developer or agent while preserving uncertainty and existing 0.22
machine behavior.

## Outcome

For one review, RepoPilot will produce one canonical proof projection that
answers:

1. what changed at a supported contract level;
2. what is proven broken, what needs review, and what was not assessed;
3. which files and consumers were actually covered;
4. which verification obligations passed, failed, or remain unavailable;
5. whether declared intent and critical paths drifted;
6. what exact next action is required.

The projection will be available consistently in the existing CLI, JSON,
Markdown, HTML, SARIF, MCP, GitHub Action, and AI-context surfaces. A bounded
receipt will make the inputs and limitations replayable without uploading
source code or invoking an implicit LLM.

## Goals

### 1. Canonical proof and receipt

- Reuse the existing `ChangeProof` as the only verdict source.
- Add an additive evidence receipt containing analyzer version, report schema,
  workspace revision, configuration/provenance hashes, requested/analyzed/
  excluded/unsupported scope, proof obligations, reason codes, and next action.
- Give every receipt a deterministic projection hash and an explicit
  `matched`, `stale`, `unsupported`, `invalid`, or `unavailable` replay state.
- Keep repository-controlled snippets redacted/escaped according to current
  path and output contracts.

### 2. First-screen Proof Card

- Put verdict, strongest reason, scope coverage, gate/exit behavior, and one
  next action before detailed findings in human reports.
- Use the same reason codes and counts in JSON, Markdown, HTML, SARIF, MCP,
  Action, and AI context.
- Keep legacy readiness fields and exit codes unchanged during the 0.x window.
- Make empty, partial, stale, unsupported, and unavailable states visible.

### 3. Evidence-backed contract packs

Promote contract families only through a repeatable pack containing:

- a positive control;
- a negative control;
- an unsupported/unknown boundary case;
- full, changed, cold-cache, and warm-cache regression coverage;
- one pinned real-repository observation;
- one controlled mutation or independent oracle where available;
- explicit limitations and rule lifecycle metadata.

Initial families are imports/exports, dependencies, delivery/auth boundaries,
taint-lite source-to-sink flows, and proof obligations. A missing pack keeps a
family `limited` or `unavailable`; it cannot create `VERIFIED`.

### 4. Safe rollout

- Preserve the existing advisory shadow path.
- Keep scoped blocking opt-in and allowlisted, with artifact provenance and
  documented rollback.
- Permit blocking only for rules with current fixtures, reviewed expectations,
  and a release-scoped evidence decision.

### 5. Scientific and release evidence

- Keep zoo, recall, real-history, mutation, and differential results separate.
- Obtain independent labels before making precision/recall/utility claims.
- Record exact unavailable identities instead of scoring them as pass.
- Close compatibility, dependency-advisory, platform-install, and publication
  recovery evidence before tagging 0.23.

## Non-goals

- No version bump, `v0.23.0` tag, package publication, or release workflow run
  in the feature phase.
- No new top-level CLI command.
- No network upload of source, remote execution, implicit command execution, or
  LLM-generated verdict.
- No claim of human usefulness without a separate human/operator study.
- No broad recall or production-precision claim from the current 6/54 rule
  evidence, pending holdout labels, or synthetic mutation results.

## Proposed architecture

### A. Canonical record boundary

The existing proof builder remains the sole decision engine. A new additive
receipt/projection layer will consume the proof and existing coverage,
verification, intent, ownership, and provenance records. Renderers must not
derive their own verdict or silently upgrade unknown coverage.

The receipt is a data projection, not a second persisted source of truth. It is
valid only for the recorded workspace revision and configuration hashes.

### B. Projection adapters

The existing renderer entry points will receive the same receipt projection.
Parity tests will compare verdict, reason codes, coverage, obligations, replay
state, and next action across every supported format. Large details remain
bounded and expandable through existing output options.

### C. Replay boundary

Replay accepts a local receipt or stored analysis handle and checks revision,
configuration, analyzer, and path-root compatibility before returning evidence.
Mismatch or unsupported replay is a structured result, never a best-effort
`verified` result. MCP explain tools and CLI detail output will use the same
boundary.

### D. Evidence harness

The existing sandbox, real-history, differential, zoo, recall, and release
scripts remain separate producers. A report aggregator may summarize them, but
it must retain corpus, protocol, label state, denominator, and unavailable
reasons for each track.

## Implementation slices

1. **Receipt contract:** define the additive receipt model, projection hash,
   replay states, and compatibility tests.
2. **Proof Card parity:** wire the receipt into CLI/JSON/Markdown/HTML/SARIF,
   MCP, Action, and AI-context projections.
3. **Replay:** implement local receipt validation and stale/unsupported paths;
   add CLI/MCP tests without a new top-level command.
4. **Contract packs:** freeze fixtures and real/mutation evidence for each
   initial family; keep unsupported forms explicit.
5. **Policy rollout:** exercise shadow/advisory/opt-in blocking on a bounded
   policy and verify rollback.
6. **RC evidence:** run full/changed/cache parity, quality, compatibility,
   platform, package, and release recovery checks; write a decision record.

Each slice is a separate reviewable PR. A failed slice leaves the previous
behavior intact and does not authorize a version bump.

## Testing and acceptance

### Functional

- Empty diff, partial coverage, stale revision, unsupported import, malformed
  source, invalid receipt, and unavailable verification all render distinct
  states.
- The same receipt produces identical verdict/reasons/coverage in every output
  surface.
- Two findings with the same stable ID but different evidence locations remain
  distinct.

### Security and resilience

- Receipts and reports remain root-confined, redacted, escaped, deterministic,
  and bounded.
- Replay refuses changed revisions, changed configuration, path escapes,
  malformed input, and unsupported schema versions.
- Timeouts terminate child processes and leave a receipt with the failure
  classification.

### Evidence

- Every promoted contract pack has positive, negative, and boundary controls.
- Zoo labels, holdout labels, and mutation outcomes remain separate datasets.
- Full/changed and cold/warm parity is measured where claimed.
- Release evidence includes all supported platform install/run checks and
  public-channel verification; skipped checks remain explicitly unavailable.

### Release boundary

The development line can be called `0.23 candidate-ready` only when the
canonical proof/receipt slices, contract packs, compatibility matrix, and
quality evidence are complete enough for review. `v0.23.0` remains blocked
until no known P1 release blocker remains, the final tag workflow passes, and
post-publication verification converges across supported channels.

## Known open risks

- Current default-profile zoo evidence covers only 6 of 54 registered rules.
- Six real-history cases remain unlabeled; 18 differential test comparisons
  lack exact test-node identity.
- Full release verification and cross-platform install parity are not yet
  proven for a future 0.23 tag.
- `cargo audit` currently reports the allowed `anyhow` unsoundness advisory
  RUSTSEC-2026-0190 through the wit dependency graph.
- Human comprehension, demand, and time savings remain unmeasured.
