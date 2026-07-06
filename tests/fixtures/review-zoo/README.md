# Review-zoo: differential safe/unsafe fixtures

A companion to [`tests/fixtures/review/`](../review/README.md)'s golden
harness. Where that harness proves one true-positive fixture per signal
family, this one proves the **precision boundary**: for the same delta
family, a `safe/` variant that must produce zero review signals sits next to
an `unsafe/` variant that must produce the expected signal — a minimal-edit
contrast, not two unrelated scenarios. Consumed by
[`tests/review_zoo.rs`](../../review_zoo.rs).

## Layout

```
tests/fixtures/review-zoo/<family>/<scenario>/{safe,unsafe}/
  before/          file tree committed as the baseline (HEAD)
  patch/
    rationale.md   required, non-empty — names the exact edit and why it is/isn't expected to signal
  after/           file tree overlaid on top, left uncommitted (== the diff)
  expected.json    { description, expect[] }   (expect[] only required for unsafe/)
```

The harness commits `before/`, overlays `after/` uncommitted (exactly like the
golden-fixture harness), then runs the real binary. `safe/` fixtures are
asserted to produce **zero** review signals unconditionally — no
`expected.json` `expect` array needed, no per-family exceptions. `unsafe/`
fixtures are asserted against `expected.json`'s `expect` array using the same
partial-match contract as the golden harness (see
[`tests/fixtures/review/README.md`](../review/README.md#expectedjson-contract)):
match only on stable fields (`bucket`, `family`, `kind`, `path`, `headline`).

`patch/rationale.md` is the fixture's documentation — why this exact edit is
safe or unsafe for this delta family, and (for `safe/`) why no *other* family
fires either. It is required and checked non-empty so it can't decay into a
placeholder; it is not applied as a literal patch by the harness (there is no
patch-apply machinery here — `before/` + `after/` already fully encode the
diff, exactly as in the golden-fixture harness).

## Coverage

| Family | Scenario | Safe reaches | Unsafe reaches |
|--------|----------|--------------|-----------------|
| `boundary` | `access-control` | new file at an ordinary path (`src/utils/format.ts`) | new file under `src/auth/**` |
| `behavioral` | `network-call` | a pure helper with no I/O | an added `fetch()` call |
| `taint` | `sql-injection` | tainted input reaches a safe ORM call | tainted input reaches raw SQL via string concatenation |

New fixtures are discovered automatically — drop a `<family>/<scenario>/{safe,unsafe}/`
directory in and it's picked up, no registration needed. A dedicated test
asserts the family set stays a superset of `{boundary, behavioral, taint}`.
