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

## Labeler disclosure

The labels were made by Claude (Anthropic, model Opus 5.5) working for the
maintainer, who reviews them before any public claim. The same model wrote
RepoPilot's integrity detectors, which can bias both what is noticed and how a
kind is read. Treat every number as single-labeler and exploratory.
