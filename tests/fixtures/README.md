# Test fixtures

Committed inputs and golden snapshots for RepoPilot's integration tests. Each
family is discovered automatically by its harness — adding a fixture rarely
needs a code change.

| Family | What it holds | Harness |
|--------|---------------|---------|
| [`rules/`](rules/) | Per-rule true-/false-positive projects (`<rule_id>/{true_positive,false_positive}/` + `expected.json`) | [`rule_eval_fixture_coverage.rs`](../rule_eval_fixture_coverage.rs) |
| [`review/`](review/) | `review` change-signal before/after scenarios (see [`review/README.md`](review/README.md)) | [`review_golden_fixtures.rs`](../review_golden_fixtures.rs) |
| [`golden/`](golden/) | Rendered-output snapshots (`scan-*`, `ai-context-*`) | [`output_golden_fixtures.rs`](../output_golden_fixtures.rs), [`ai_context_golden.rs`](../ai_context_golden.rs) |
| [`projects/`](projects/) | Small committed sample repos scanned by tests (e.g. `ai-context-sample`) | [`ai_context_golden.rs`](../ai_context_golden.rs) |
| [`reports/`](reports/) | Historical scan JSON per schema version (`scan-v0NN.json`) | [`report_schema_contract.rs`](../report_schema_contract.rs) |
| [`risk/`](risk/) | Risk-scoring calibration corpora | [`risk_calibration.rs`](../risk_calibration.rs) |

## Adding a rule fixture

Create `rules/<rule_id>/` (the directory name **must** be a registered rule id —
the harness rejects unknown ids) with an `expected.json`:

```json
{
  "fixtures": [
    { "path": "false_positive", "expected_rule_ids": [] },
    { "path": "true_positive", "expected_rule_ids": ["<rule_id>"] }
  ]
}
```

- **`true_positive/`** — the smallest tree that *should* fire the rule.
- **`false_positive/`** — a look-alike that must **not** fire it: the known
  safe shape (a keyword in a string/comment, a parameterized query, a value
  exactly at a threshold). This is where a rule earns its precision.
- Keep each tree minimal so unrelated rules stay quiet. Stable rules are gated
  (they must ship both a TP and an FP, documented `false_positive_notes`, and
  clean findings); Preview/Experimental rules are pinned but not gated. The
  aggregate test also fails on missing/unexpected findings, contract violations,
  and unstable ids across every fixture.
- Config-only rules that emit nothing under the default scan (e.g.
  `architecture.layer-violation`) live in
  [`architecture_opt_in_rules.rs`](../architecture_opt_in_rules.rs) instead.

## Adding a review fixture

See [`review/README.md`](review/README.md): drop a
`review/<family>/<scenario>/` directory with `before/`, an optional `after/`,
and an `expected.json` of `expect`/`forbid` constraints (plus an optional
`delete` list for deletions and renames).

## Updating goldens

Goldens are regenerated, never hand-edited. Two separate bless switches:

```sh
# Rendered output + AI context snapshots (golden/)
REPOPILOT_UPDATE_GOLDEN=1 cargo test --test output_golden_fixtures --test ai_context_golden

# The generated rules reference doc (docs/rules-reference.md)
REPOPILOT_BLESS=1 cargo test --test rules_reference_doc
```

Review the diff before committing — a golden change is a behavior change.

## Conventions

- **No real secrets.** Use structurally realistic fakes; placeholders like
  `<OPENAI_API_KEY>`, `${...}`, or `your-api-key` must not trip a rule unless
  that rule is meant to catch them.
- **Deterministic.** No timestamps, wall-clock, or absolute machine paths in a
  committed snapshot. Normalize volatile fields in the harness instead (the AI
  context golden zeroes the scan duration and pins the cache-status line).
- Scanning a fixture writes a gitignored `.repopilot/cache/` under it — that is
  expected and never committed.

## Running only the fixture tests

```sh
cargo test --test rule_eval_fixture_coverage --test review_golden_fixtures \
           --test ai_context_golden --test output_golden_fixtures
```
