# Core Visibility Engine

RepoPilot's visibility engine is the first step toward making the scanner behave
like a repository decision layer rather than a noisy list of static-analysis
matches.

## Problem

Raw audit findings are useful for rule development, but they are too noisy for
daily development. A production-risk report should not be dominated by:

- missing-test suggestions
- large-file thresholds
- long-function thresholds
- script-level `process.exit`
- low-severity style or maintainability signals

## Model

The core flow is:

```text
Finding
  -> FindingIntent
  -> FindingVisibilityDecision
  -> visible finding OR hidden suggestion summary
```

The important types are:

```rust
FindingIntent
FindingVisibilityDecision
HiddenSuggestionSummary
FindingVisibilityProfile
```

`FindingIntent` describes why the finding exists from the product perspective.
`FindingVisibilityDecision` describes whether it belongs in the default report.
`HiddenSuggestionSummary` preserves what was hidden so default output remains
transparent.

## Default profile

The default profile keeps findings that are useful for normal development and CI:

- high-confidence security risks
- high-severity runtime risks outside script/tooling boundaries
- import-graph risks with concrete repository evidence
- high-priority actionable risks outside broad maintainability heuristics
- high-severity/high-confidence release risks

It hides:

- testing gaps
- maintainability suggestions
- TODO/FIXME/HACK, long-function, and complex-file heuristics
- low-severity informational findings
- process exits in scripts/tools/CI guard files

## Strict profile

Strict profile keeps the full raw audit output and clears hidden suggestion
breakdowns because nothing is hidden.

Use it for:

- codebase cleanup
- refactoring passes
- rule authoring
- pre-release deep audits
- calibration work

## Why hidden summaries are part of the core model

Without hidden summaries, a quiet default report can look like RepoPilot missed
information. With hidden summaries, the report communicates:

```text
I found more, but those findings are strict-mode suggestions.
Here is what they were grouped by rule, intent, and reason.
```

This is the difference between a blind filter and a trustable decision engine.
