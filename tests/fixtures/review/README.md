# Review signal golden fixtures

Before/after corpus for the `review` change signals, consumed by
[`tests/review_golden_fixtures.rs`](../../review_golden_fixtures.rs). Unit tests
next to each detector prove the true/false-positive rules in isolation; these
fixtures prove the signals survive the real pipeline — a git diff parsed from a
temp repo, folded into the tiered view, and serialized through
`repopilot review --format json`.

## Layout

```
tests/fixtures/review/<family>/<scenario>/
  before/        file tree committed as the baseline (HEAD)
  after/         OPTIONAL file tree overlaid on top, left uncommitted (== the diff)
  expected.json  { description, label, delete[], expect[], forbid[] }
```

The harness commits `before/`, overlays `after/` (new files are added, shared
paths overwritten), removes any paths listed under `delete`, runs the real
binary, then checks the constraints. New fixtures are discovered automatically —
just drop a directory in.

### Deletions and renames

`expected.json` may carry an optional `delete` array of repo-relative paths that
are removed from the working tree **after** the overlay, so review sees them as
deletions. `after/` is optional — a pure-deletion scenario has only `before/`
plus `delete`. A **rename** is `delete` of the old path plus the new path in
`after/` (working-tree diffs are not rename-detected, so a move reads as a
delete + an add).

## `expected.json` contract

`expect` and `forbid` are arrays of **partial-match constraints** over the
unified tiered signals. Each constraint is an object of `field: value` pairs;
a signal matches when every named field is equal. Match only on stable fields —
`bucket`, `family`, `kind`, `path`, `headline` — never timing, ids, or blast
radius.

- every `expect` constraint must match **at least one** emitted signal,
- every `forbid` constraint must match **none**.

`bucket` is the tier the signal landed in: `definitely`, `maybe`, or `noise`.
`label` documents the fixture's intent (`expected_true_positive`,
`expected_false_positive`, `ambiguous`, `needs_real_repo_case`) and is not
enforced.

```json
{
  "description": "why this change should produce the signal",
  "label": "expected_true_positive",
  "expect": [
    { "bucket": "definitely", "family": "taint", "kind": "taint.sql",
      "path": "src/handler.ts", "headline": "untrusted input reaches raw SQL" }
  ],
  "forbid": [
    { "family": "volume" }
  ]
}
```

## Seed coverage

One proven-firing fixture per signal family (the vertical slice the matrix grows
from):

| Family | Scenario | Tier | Demonstrates |
|--------|----------|------|--------------|
| `taint` | `sql-injection` | definitely | request input concatenated into raw SQL |
| `boundary` | `access-control` | definitely | a new `src/auth/**` file |
| `behavioral` | `network-call` | maybe | `fetch()` added in an ordinary file |
| `algorithmic` | `nested-loop` | maybe | a nested loop introduced in a function |
| `deletions` | `remove-auth-guard` | definitely | deleting a `src/auth/**` file still flags the boundary |
| `deletions` | `remove-test-file` | definitely | deleting a `*.test.ts` removes coverage |
| `renames` | `move-auth-entrypoint` | definitely | moving an auth file flags both old and new path |
| `refactor` | `no-behavior-change` | — | a local-variable rename stays silent |
| `noise` | `large-diff` | noise | >5 files / >200 lines of data trips only the volume note |
