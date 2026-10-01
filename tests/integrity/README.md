# Integrity corpus

Public pull requests that modify test code, labeled for the ways a change can
weaken the checks that judge it. It measures how often that happens and what
RepoPilot catches (v0.24 Phase D, RP24-022). Results are exploratory.

## Sampling

`scripts/integrity_corpus.py sample` searched GitHub for pull requests that
were created 2026-06-01..2026-09-28, merged with an approving review, in
public, non-fork, non-archived repositories with at least 100 stars, with at
most 60 changed files, and with at least one modified, renamed, or removed test
file (path rules in `scripts/integrity_corpus_paths.py`).

- **agent**: up to 20 PRs each authored by the Copilot coding agent, Devin, or
  Jules apps, labeled `codex`, or carrying "Generated with Claude Code" in the
  body; at most 2 per repository.
- **human**: up to 2 PRs per agent repository from a `User` account with no
  agent marker in the body. Humans may still have used AI assistance; the
  group means "no agent marker", not "no AI".

The manifest pins every PR by base and head SHA. Search ranking decided which
PRs were found first, so the sample is not uniformly random.

## Labels

`labels.toml` records, per PR, the weakening `kinds` present and a `verdict`.

| Kind | Present when |
|---|---|
| `skip-added` | A test that ran before is skipped, marked expected-to-fail, turned into a todo, or ignored |
| `focus-added` | A focus marker (`.only`, `fit`, `fdescribe`) is committed |
| `test-removed` | A test case that existed before is gone and was not moved elsewhere in the PR |
| `test-substituted` | A test was removed and a new one in its place checks different or weaker behavior |
| `assertion-removed` | A test that remains checks less: an assertion deleted, or made looser (exact → partial, value → defined) |
| `assertion-trivialized` | An assertion was replaced by one that cannot fail |
| `expectation-rewritten` | An expected value changed in the same PR as the code it checks |
| `suppression-added` | A lint, type, or coverage suppression was added anywhere |
| `gate-relaxed` | CI or tool configuration lets a failing check pass or runs fewer tests |

Verdicts:

- `weakened`: at least one kind is present and nothing in the PR explains it
  as a deliberate change of behavior.
- `justified`: kinds are present, but the PR removes or changes the behavior
  under test, moves or rewrites the test with equal checks, or states a reason.
- `none`: no kind is present.

Labels are made from the PR title and test/gate diffs before any RepoPilot run
on the corpus, using `integrity_corpus.py show --compact ID`: hunks that remove
lines are shown in full; purely additive hunks show only lines that use skip,
focus, suppression, or gate vocabulary. Weakening that is purely additive and
avoids that vocabulary would be missed by the labeler as well.

## Splits

- **Development (`ic-*`, `manifest.toml`)**: the sample above. After its
  first evaluation, the detectors were changed to fix false alarms found on it.
  Its catch table therefore overstates precision.
- **Held-out (`ih-*`, `manifest-holdout.toml`)**: 98 merged, approved PRs
  created 2026-03-01..05-31, sampled the same way (up to 10 PRs per agent).
  They were sampled after the detectors were tuned, and labeled and committed
  before any RepoPilot run on them. The held-out table is the one to quote.
  Two changes to the labeling were fixed before its labels were made:
  - `show --compact` also lists added lines in other files that look like a
    lint, type, or coverage suppression. The development labels missed four
    such suppressions, because the view did not show them.
  - A renamed test whose body now checks different behavior is labeled
    `test-substituted`, as the definition says. The development labels often
    recorded these as `expectation-rewritten` only.
- **Closed (`icc-*`, `manifest-closed.toml`)**: agent PRs created
  2026-06-01..09-28 that were closed without merge after discussion (up to 15
  per agent). This split tests whether weakening is more common in changes
  that reviewers turned down.

## Evaluation and reconciliation

`integrity_corpus.py evaluate` rebuilds each PR as a two-commit repository,
from the merge base to the head, and runs `repopilot review` on it. `report`
writes [REPORT.md](REPORT.md) and the per-PR signal kinds in `results.json`, so
the report regenerates without the gitignored cache. The development split was
evaluated with commit `5ff13d16`. The held-out and closed splits were evaluated
once, with commit `b061d06f`. Detection code is the same in both commits.

Blind labels stay the primary table. A second table applies reconciliations.
Each one is a label that missed evidence which is objectively in the PR, and is
recorded in `reconciled_kinds` with its reason:

- development: four suppressions in source files that the labeling view did
  not show;
- held-out: one suppression that was in the view but past the lines the
  labeler read. After evaluation, every held-out and closed view was re-read in
  full for kind vocabulary, not only the PRs where RepoPilot and the label
  disagree. This was the only label it changed.

Only missed evidence is reconciled. Judgment calls stay as labeled, even where
the rubric could be read either way.

`behavioral.test-deleted-or-emptied` reports a whole test file that was
deleted or emptied. `integrity.test-removed` reports test cases removed from a
file that still has tests. The report has a combined row for either signal.

## Labeler disclosure

The labels were made by Claude (Anthropic, model Opus 5.5) working for the
maintainer, who reviews them before any public claim. The same model wrote
RepoPilot's integrity detectors, which can bias both what is noticed and how a
kind is read. Treat every number as single-labeler and exploratory.
