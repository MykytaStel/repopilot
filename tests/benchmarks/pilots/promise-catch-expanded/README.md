# Expanded Promise `.catch()` discovery frame

This is exploratory discovery data from one text-ranked GitHub PR search run on 2026-10-24. It is separate from the preregistered 59-PR feasibility screen at `../promise-catch/`.

## Contents

- `manifest.toml`: query, frame size, per-repository counts, and limitations.
- `candidate_pool.json`: 197 PR identifiers, titles, and URLs from 17 repositories.
- `screening.json`: 34 PRs where a patch hunk contains `.catch()`, 26 with at least one match in a source-like path according to a simple filename heuristic, and 12 patch-fetch connector errors.

The screen is intentionally not a label set. Hunk/path matches can be adjacent context, test harnesses, docs, large merge PRs, unchanged callbacks, deliberate recovery, or error paths that preserve failure. Ten diffs received a manual spot check to understand these failure modes; that is not enough to adjudicate or exhaustively classify the frame.

## Use

Use this frame to find candidate examples and safe controls for a future pilot. Before producing any RepoPilot baseline, screen each case against the exact callback diff, record immutable base/head/merge SHAs and intent context, freeze repository-level development/evaluation splits, and obtain independent labels while RepoPilot output is hidden.

Do not report precision, recall, prevalence, or production accuracy from this file. No PR code or tests were executed, no labels were collected, and no RepoPilot outputs were generated.

