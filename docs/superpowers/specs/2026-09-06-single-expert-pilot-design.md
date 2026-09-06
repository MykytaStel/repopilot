# Single-Expert Differential Pilot Design

## Problem

RepoPilot now collects repeated, baseline-aware observations for the pinned
real-history holdout, but the preregistered quality protocol requires two
independent human annotations. The project currently has one available human
reviewer. Treating a model assessment or a second pass by the same person as
independent ground truth would overstate the evidence.

## Goal

Add an explicit `single-expert-pilot-v1` workflow that lets one named reviewer
label the existing differential artifact, validates the labels and provenance,
and emits exploratory case-level metrics without changing or weakening the
existing `dual-independent-adjudication-v1` protocol.

## Non-goals

- Do not alter the preregistered dual-review manifest or its validators.
- Do not claim production, language-wide, or general recall/precision.
- Do not infer labels from RepoPilot findings, baseline failures, or upstream
  merge status.
- Do not call a model-assisted assessment independent human annotation.
- Do not add a new top-level Rust CLI command; this workflow remains in the
  benchmark Python tooling.

## Data flow

```text
differential-run-v2.json
        |
        v
pilot-template -> pilot-assessment.toml -> pilot-validate -> pilot-score
                                      |
                                      v
                         exploratory-pilot-metrics.json
```

The template copies case identity and aggregate baseline statuses only. It
never copies RepoPilot rule IDs or evidence keys, preserving a blinded review
surface. The completed assessment records the reviewer name, label,
expected rule IDs, and a rationale for every case.

## Artifact contracts

The pilot worksheet uses schema version 1 and protocol
`single-expert-pilot-v1`. Its top-level fields are `schema_version`, `corpus`,
`protocol`, `reviewer`, `manifest_sha256`, `differential_artifact_sha256`,
`blinded = true`, `assessment_mode = "single-expert-exploratory"`, and one
`case` table per holdout case. Each case contains pinned identity, baseline
statuses, `label`, `expected_rule_ids`, and `rationale`.

Allowed labels remain `defect-present`, `no-defect`, and `uncertain`. A
`defect-present` case must name at least one known rule ID. `uncertain` cases
are excluded from confusion counts rather than silently treated as negative.

The score artifact records input hashes, per-case observed novel rule IDs,
outcomes, confusion counts, Wilson 95% intervals, deterministic review count,
and timing/resource summaries already captured by the differential artifact.
Its scope is explicitly `single-expert exploratory pilot` and its limitation
states that the result is not independent validation or a general estimate.

## CLI surface

Extend `scripts/differential.py` with:

```text
pilot-template --artifact ARTIFACT --reviewer NAME --output WORKSHEET
pilot-validate --artifact ARTIFACT --pilot WORKSHEET
pilot-score --artifact ARTIFACT --pilot WORKSHEET --output SCORE
```

All three commands first validate the differential artifact against its pinned
manifests. `pilot-score` validates the worksheet again before calculating any
metric and refuses unknown rules, hash drift, missing cases, missing rationale,
or invalid labels.

## Measurement semantics

For each labeled case, the predicted positive set is the set of novel in-diff
evidence identities from the differential artifact. A defect-present case is a
true positive only when an observed novel rule ID intersects its expected rule
IDs. A no-defect case with any novel evidence is a false positive. Uncertain
cases are excluded. The artifact also reports the number of cases with
deterministic repeated evidence and the median wall time for baselines and
reviews; it does not invent decision-latency data that the collector did not
capture.

## Testing and documentation

- Unit-test worksheet rendering, blinding, validation failures, rule matching,
  uncertain exclusion, and Wilson interval output.
- Add CLI examples and the single-expert limitation to
  `tests/benchmarks/README.md`.
- Add the evidence boundary and RP23-014 status to
  `docs/engineering/v0.23-evidence-ledger.md` and `CHANGELOG.md`.
- Keep generated run artifacts under `.repopilot/evidence/`, which is ignored
  and never committed as release evidence.
