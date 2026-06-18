# Real-repo validation zoo

The zoo runs RepoPilot against a curated set of **real** open-source
repositories spanning all 9 AST languages and their frameworks, so we can see
how rules behave on real-world diversity — not just the minimal synthetic
fixtures under `tests/fixtures/rules/`.

It is the systematic version of the manual "run RepoPilot on a few real repos
and hunt false positives" loop that produced the 0.18 increment fixes.

## What's tracked vs. ignored

- **Tracked (committed):** [`manifest.toml`](manifest.toml) (repos + pinned
  SHAs) and `snapshots/*.json` (per-repo default-visible finding counts +
  fingerprints, and strict counts). These are small.
- **Ignored:** the actual repo checkouts live in `/.zoo/` (gitignored) and are
  **never committed**. They are cloned on demand.

## The loop

```bash
# 1. Clone every repo at its pinned SHA into .zoo/ (shallow, network).
python3 scripts/zoo.py clone

# 2. Scan each repo (default + strict) and compare to committed snapshots.
python3 scripts/zoo.py scan          # exits non-zero on drift
python3 scripts/zoo.py scan --bless  # accept changes -> rewrites snapshots

# 3. Aggregate into a per-rule triage table (drives false-positive work).
python3 scripts/zoo.py report
```

Other helpers: `python3 scripts/zoo.py list` (manifest table),
`python3 scripts/zoo.py verify-pins` (clones match pins), and `--only a,b`
on `clone`/`scan` to work a subset.

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
