# Rule quality gate

RepoPilot rules should move through a lifecycle before they become stable or default-visible.

## Lifecycle

```text
experimental -> preview -> stable
```

A rule can be experimental while the detector is useful but still noisy. A preview rule can appear in strict/profile-driven scans when it has useful evidence but incomplete fixture coverage. A stable rule can be default-visible only when it passes the quality gate below.

## Stable/default-visible gate

A rule should not be stable and default-visible unless it has:

- rule metadata: title, description, recommendation, lifecycle, confidence, false-positive notes;
- at least one true-positive fixture;
- at least one false-positive fixture or a documented reason why that does not apply;
- stable finding IDs and deterministic output;
- evidence that points to a concrete file/line/scope;
- docs URL for high/critical or P0/P1 findings;
- clean contract validation in `repopilot inspect eval-rules --format json`.

## Default visibility policy

Default scans should prioritize:

- security risks;
- runtime risks;
- high-confidence architecture/review risks;
- actionable findings that a maintainer can fix now.

Strict scans may include:

- long files/functions;
- testing gaps;
- TODO/FIXME/HACK markers;
- broad text heuristics;
- experimental framework signals.

## Heuristic rules

Text heuristics are allowed, but they should be honest about their source and confidence. A text heuristic should not pretend to be AST-backed. If a rule does not use parsed facts, its metadata should reflect that and its default visibility should be conservative.

## 0.13 focus list

Strengthen before expanding:

- `security.secret-candidate`;
- private key / committed env checks;
- Rust panic/unwrap/runtime risks;
- JavaScript/Python/Go runtime exit risks;
- import graph / blast-radius rules;
- review diff rules and baseline status.
