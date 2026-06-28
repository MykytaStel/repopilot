# Real-repo validation zoo

The zoo runs RepoPilot against a curated set of **real** open-source
repositories spanning all 9 AST languages and their frameworks, so we can see
how rules behave on real-world diversity — not just the minimal synthetic
fixtures under `tests/fixtures/rules/`.

It is the systematic version of the manual "run RepoPilot on a few real repos
and hunt false positives" loop that produced the 0.18 increment fixes.

## What's tracked vs. ignored

- **Tracked (committed):** [`manifest.toml`](manifest.toml) (repos + pinned
  SHAs), `snapshots/*.json` (per-repo default-visible finding counts +
  fingerprints, and strict counts), and `expectations/*.toml` (per-repo
  human-reviewed dispositions). These are small.
- **Ignored:** the actual repo checkouts live in `/.zoo/` (gitignored) and are
  **never committed**. They are cloned on demand.

## Snapshots vs. expectations

Two separate layers, deliberately kept apart:

| Layer | File | Answers | Written by |
|-------|------|---------|------------|
| **Snapshot** | `snapshots/<repo>.json` | *What did RepoPilot emit?* (counts + fingerprints) | the scanner (`scan`, `--bless`) |
| **Expectation** | `expectations/<repo>.toml` | *How did a human review those emissions?* | a human reviewer |

A green snapshot means "the output did not change." A green expectation set
additionally means "every default-visible finding was reviewed and classified."
**Labels never change analyzer output.** `--bless` rewrites snapshots; it
**never** touches expectation files.

## The loop

```bash
# 1. Clone every repo at its pinned SHA into .zoo/ (shallow, network).
python3 scripts/zoo.py clone

# 2. Scan: compare snapshots AND validate labels (drift + expectations report
#    on separate channels). Exits non-zero on either.
python3 scripts/zoo.py scan
python3 scripts/zoo.py scan --bless   # accept snapshot drift only (not labels)

# 3. Triage: print unlabeled default findings + a copy-paste TOML skeleton.
python3 scripts/zoo.py triage
python3 scripts/zoo.py triage --only excalidraw

# 4. Assign a human disposition: paste the skeleton into expectations/<repo>.toml
#    and replace REVIEW_REQUIRED + the empty reason. Re-run `scan` to validate.

# 5. Report: per-rule table + reviewed signal-quality summary.
python3 scripts/zoo.py report
```

Other helpers: `python3 scripts/zoo.py list` (manifest table),
`python3 scripts/zoo.py verify-pins` (clones match pins), and `--only a,b`
on `clone`/`scan`/`triage` to work a subset. `triage --write` creates a
REVIEW_REQUIRED skeleton file only for a repo that has none — it never guesses a
disposition and never overwrites existing labels.

## Dispositions

Every expectation entry carries a closed-enum `disposition` and a non-empty
human `reason`:

- **`actionable`** — a real risk or design issue the rule intends to surface.
- **`valid-but-accepted`** — correctly detected structural fact, but the upstream
  repo intentionally accepts it (a design trade-off). Not a detector false
  positive; just less actionable for that repo.
- **`false-positive`** — does not represent the rule's claimed condition, or
  lacks the context to justify default visibility. This is **calibration debt**:
  it is reported prominently but, in this PR, does **not** fail the gate and is
  **never** turned into a suppression.

Optional fields: `line`, `line_end` (required to disambiguate two findings that
share a stable id), plus free-form `issue`, `notes`, `reviewed_by`, `expires`.

## Default exhaustive vs. strict selective

- **Default profile is exhaustive.** Every default-visible live finding must map
  to exactly one expectation and vice-versa. Unlabeled findings, stale
  expectations, duplicates, malformed entries, and rule/path mismatches all fail.
- **Strict profile is selective.** Strict output can run to hundreds of broad
  findings, so only listed strict expectations are checked — each must still match
  exactly one live finding (a *recall anchor*). Unlisted strict findings are
  allowed. Anchors pin intentionally downgraded signals (e.g. the public Algolia
  DocSearch key, a test-support panic, a CLI command-handler `process.exit`, a
  Python `assert`) so a downgrade can never silently become a full suppression.

## Common tasks

- **Add a new repo:** follow "Adding or bumping a repo" below, then
  `triage --only <name>` and create `expectations/<name>.toml` (required even when
  the repo has zero default findings).
- **Update a pin:** bump the `sha`, re-clone with `--force`, `scan --bless`, then
  reconcile labels — new/changed findings show as unlabeled (`triage`), removed
  findings show as stale expectations to delete.
- **A new default finding appeared:** `scan` fails with `UNLABELED DEFAULT
  FINDING`; run `triage`, classify it, add the entry.
- **Remove a stale expectation:** `scan` fails with `STALE EXPECTATION`; delete
  the entry whose finding no longer exists.
- **Why `false-positive` is not a suppression:** expectations describe current
  behavior; they do not change what the analyzer emits. A false positive stays
  visible and is surfaced as calibration debt to be fixed in the rule, not hidden.
- **Why `--bless` never updates labels:** snapshots and human review are separate
  concerns. Re-blessing accepts a *measured* output change; a human must still
  review what changed.

`scan` builds the current workspace once with Cargo, resolves the executable
Cargo produced, validates its version/report metadata, and then reuses that
binary for every repo/profile scan. Existing `target/release/repopilot` or
`target/debug/repopilot` artifacts are never selected just because they already
exist.

To intentionally compare snapshots with an external binary:

```bash
python3 scripts/zoo.py scan --repopilot /path/to/repopilot
python3 scripts/zoo.py scan --repopilot /path/to/repopilot --allow-version-mismatch
```

External binaries are provenance-checked too. A version mismatch fails unless
`--allow-version-mismatch` is supplied.

## As a regression gate

[`tests/zoo_regression.rs`](../zoo_regression.rs) wraps `zoo.py scan` and is
**opt-in** — it no-ops unless `REPOPILOT_ZOO=1`, because it needs the repos
cloned locally. It is not part of the default `cargo test --all` / CI run.

```bash
python3 scripts/zoo.py clone
REPOPILOT_ZOO=1 cargo test --test zoo_regression      # fail on drift
REPOPILOT_ZOO_BLESS=1 cargo test --test zoo_regression # re-bless via test
```

Treat a drift failure like a golden-snapshot failure: inspect the diff, and if
the rule change was intentional, re-bless.

## Adding or bumping a repo

1. Pick a permissively-licensed, well-known, medium-sized repo for a
   language/framework we want more real coverage of.
2. Get a real, reachable commit SHA: `git ls-remote <url> HEAD`.
3. Add a `[[repo]]` block to `manifest.toml` (name, url, sha, language,
   framework, license, notes).
4. `python3 scripts/zoo.py clone --only <name>` then
   `python3 scripts/zoo.py scan --only <name> --bless` to create its snapshot.
5. Commit `manifest.toml` + `snapshots/<name>.json`.

To advance an existing pin, update its `sha`, re-clone with `--force`, and
re-bless. A SHA bump that changes findings is expected to change the snapshot.
