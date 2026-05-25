# Rule quality gate

RepoPilot rules should move through a lifecycle before they become stable or
default-visible.

## Lifecycle

```text
experimental -> preview -> stable
```

A rule can be experimental while the detector is useful but still noisy. A
preview rule can appear in strict/profile-driven scans when it has useful
evidence but incomplete fixture coverage. A rule can be stable only when it
passes the quality gate below.

## Stable/default-visible gate

A rule should not be marked `stable` unless it has:

- rule metadata: title, description, recommendation, lifecycle, confidence, and
  false-positive notes;
- at least one true-positive fixture;
- at least one false-positive fixture;
- stable finding IDs and deterministic output;
- evidence that points to a concrete file/line/scope;
- docs URL for high/critical or P0/P1 findings;
- clean contract validation in `repopilot inspect eval-rules --format json`.

The local gate reports `has_true_positive_fixture`,
`has_false_positive_fixture`, `contract_violations`, `stable_id_failures`, and
`quality_gate_failures` for each evaluated rule.

## Release and smoke gate

`inspect eval-rules` is part of the release smoke gate. Before a release, this
must pass:

```bash
repopilot inspect eval-rules --format json --output /tmp/repopilot-rule-eval.json
```

The aggregate JSON fields must be zero:

- `missing_findings`;
- `unexpected_findings`;
- `contract_violations`;
- `stable_id_failures`;
- `quality_gate_failures`.

`scripts/smoke-product.sh` and `scripts/verify-release.sh` enforce these checks
without requiring `jq`.

## Default visibility policy

Default scans should prioritize:

- security risks;
- runtime risks;
- import graph and review risks with concrete blast-radius evidence;
- actionable findings that a maintainer can fix now.

Strict scans may include:

- long files/functions;
- complex-file and broad code-quality heuristics;
- testing gaps;
- TODO/FIXME/HACK markers;
- broad text heuristics;
- experimental framework signals.

Default scans should not be dominated by weak heuristics. If a broad heuristic
cannot be made contextually precise, keep it strict-only or experimental.

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
