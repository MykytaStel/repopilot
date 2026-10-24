# First feature-first slice: Promise rejection fallbacks

**Status:** approved pilot completed as a feasibility screen; current cohort
did not meet the predeclared denominators, so no product code changed.  
**Date:** 2026-10-23

## Decision

Extend RepoPilot's existing quiet-fallback review signal to a narrow class of
changed JavaScript/TypeScript `Promise.catch` rejection handlers, and evaluate
that slice on a pinned, independently labeled real-PR pilot before widening it.
This advances one technical slice ahead of the v0.25 first-use evidence gate at
the user's explicit direction. It does not close or replace first-use evidence.

## Evidence and problem

The current `behavioral.quiet-fallback-introduced` detector visits parsed
JavaScript/TypeScript `try_statement` nodes and Python `try`/`except` nodes. It
does not inspect promise `.catch()` callbacks. The current unit suite explicitly
excludes a bare optional-import `.catch(() => null)` without a downstream
fallback.

The October agent-eval contains a synthetic `js-missing-native` task. In the
plain run, the agent added a rejected-import fallback and returned the original
bytes after image processing failed; the hidden oracle rejected that behavior.
The same change also added an empty `try/catch` around image processing, which
may already be covered by the current signal. In the plugin run, the agent
instead injected a loader and tested the transform contract with a stub. The
agent-eval report records integrity and hook outcomes, not review-only signal
observations, so this task does not prove that RepoPilot missed the behavior or
that Promise `.catch` support would add value. It is a concrete hypothesis for
the pilot, not evidence about production frequency or detector precision.

Sources: `src/review/signals/quiet_fallback.rs`,
`src/review/signals/quiet_fallback_tests.rs`, `tests/agent_eval/REPORT.md`, and
the follow-up candidates in `docs/roadmap/v0.25.md`.

## Proposed behavior

If the pilot shows incremental value, extend the existing
`behavioral.quiet-fallback-introduced` signal when all of these conditions are
proven from the parsed JS/TS diff:

- a `.catch()` rejection callback is new or changed on a promise-producing
  operation;
- the changed callback consumes the rejection without rethrowing or preserving
  a failure result; and
- the enclosing expression or function can return a fallback value as success.

Point to the changed callback and include the originating operation and
downstream fallback location when available. Phrase the message as a behavior
to verify; syntax alone cannot prove that the fallback is wrong. Keep the
signal at `maybe` confidence and non-gating. Preserve test-file exclusion,
parse-error abstention, and CLI/MCP parity. Do not flag every `.catch()` call.

The first slice does not cover `.then(onFulfilled, onRejected)`, `finally`,
custom promise wrappers, other languages, or LLM-generated explanations.

## Pilot design

Use the existing immutable-PR and dual-review workflow where it fits, extending
its collected review projection to retain stable signal kinds and evidence
locations. The current runner requires Python-only baseline commands. Add an
explicit review-only observation state for these JS/TS cases; never report an
irrelevant Python baseline as passed. Compare the new candidate with today's
detector and count only additional actionable findings as incremental value.
Do not execute repository scripts, install packages, or run arbitrary PR
checks; collect Git revisions and run RepoPilot's local static review only.

Build a separate enriched pilot from public merged PRs in JavaScript and
TypeScript repositories outside `tests/zoo`. Freeze the selection query, date
window, repository popularity snapshot, and per-repository cap before reviewing
diffs. Include changed `.catch()` candidates and matched safe controls, with
cases distributed across repositories so one project cannot dominate the
result. Pin base/head/merge SHAs. Keep the PR description and linked issue as
intent context, while hiding RepoPilot output from both reviewers.

Freeze repository-level development and evaluation splits before labels or
RepoPilot results are produced. Two reviewers label each case independently
as `harmful-fallback`, `justified-recovery`, or `uncertain`, citing the changed
behavior and the evidence for intent. A third record resolves disagreements.
If two independent reviewers are unavailable, keep the pilot exploratory and
do not report precision or recall as independently measured.

Keep all labels blinded to RepoPilot results. Run the current detector as a
baseline after labels are frozen. Tune a candidate only on development
repositories, then freeze its commit and configuration before the first
evaluation run. Report held-out counts and 95% Wilson intervals for:

- detection rate among adjudicated harmful fallbacks;
- false-alert rate among justified recoveries;
- exactness of the changed evidence location; and
- analyzable coverage by repository and language.

This enriched pilot can show whether the detector helps on the sampled pattern.
It cannot estimate how often the problem occurs in all PRs or all languages.
Keep the broader 358-PR integrity corpus as a separate dataset with its original
selection limits.

## Go/no-go for implementation and expansion

Before selecting cases, freeze minimum denominators and thresholds. The target
is at least 20 adjudicated harmful cases and 20 justified controls in evaluation,
across at least two held-out repositories (one JavaScript and one TypeScript),
plus both classes in development repositories. A candidate is worth expanding
only if it catches at least 70% of held-out harmful cases, alerts on no more
than 10% of justified controls, and points to the changed evidence location.
Keep those thresholds as pilot gates, not production-wide accuracy claims. If
the sample cannot meet the minimum denominators, report the feasibility result
and do not claim a successful detector evaluation.

First require development labels to show harmful Promise `.catch` fallbacks
that the current signal does not already cover. If none exist, close this
candidate without implementation. Otherwise, implement and tune on development,
then evaluate the frozen candidate once. Promote the slice only if the held-out
recall, false-alert, and evidence-location gates pass. A future larger corpus
or LLM knowledge work is a separate decision.

## Feasibility update — 2026-10-23

The frozen text-ranked search frame contains 59 merged PRs. Diff screening found
four production PR cases with added or changed Promise `.catch()` callbacks:
one development case and three evaluation cases across three repositories. It
cannot meet the required 40 held-out harmful/justified cases or establish both
behavior classes in development. The separate feasibility artifacts are in
`tests/benchmarks/pilots/promise-catch/`. The pilot stopped before labels and
RepoPilot baseline runs; there is no detector precision, recall, or product
implementation claim.
