# Promise `.catch()` pilot feasibility screen

This is the user-approved feature-first pilot for extending
`behavioral.quiet-fallback-introduced` to a narrow class of changed JavaScript
and TypeScript Promise rejection handlers. It is a feasibility screen, not a
precision or recall benchmark.

## Recorded protocol

- PR search snapshot: `2026-10-23T15:29:41Z`. The exact retrieval time of the
  initial repository popularity search was not captured; membership in the
  same threshold queries was revalidated at `2026-10-23T15:37:32Z`.
- Popularity searches: JavaScript repositories with more than 50,000 stars and
  TypeScript repositories with more than 30,000 stars; exact queries and the
  selected repository splits are in `manifest.toml`.
- PR search: merged PRs from 2024-01-01 through 2026-10-23, text query
  `catch fallback`, best-match ordering, maximum 10 returned per repository.
- Repositories: development uses `vercel/next.js` and `openclaw/openclaw`;
  evaluation uses `react/react`, `mrdoob/three.js`, `microsoft/vscode`, and
  `n8n-io/n8n`.
- Repository popularity thresholds and splits were selected before PR patch
  screening. The initial popularity-query retrieval time was not recorded;
  returned membership was revalidated at `2026-10-23T15:37:32Z`, after diff
  screening. Treat this as a feasibility screen with incomplete snapshot
  provenance, not a fully preregistered detector evaluation.
- `candidate_pool.json` stores the frozen search frame. `screening.json` stores
  only relevant handler candidates, near misses, pinned revisions, and short
  summaries of the PR's stated reason for change.

One PR (`vercel/next.js#100552`) remains in `candidate_pool.json` as raw query
metadata for traceability, but is excluded from `screening.json` because its
diff was opened during exploratory discovery before the frame was frozen. The
screening used GitHub metadata and diffs only. No PR code, checks, or scripts
were executed; no packages were installed.

## Feasibility result

The frozen frame contains 59 PRs. Diff screening found four production PR cases
with added or changed Promise `.catch()` callbacks, two new `.catch()` expressions
only in regression tests, and one near miss where the operation changed but the
callback did not.
Only one production case directly resembles the proposed fallback-to-success
shape; the others rethrow, log a detached failure, or perform best-effort
DevTools work. There are three production cases in evaluation and one in
development, all from only three repositories.

The predeclared evaluation minimum is 20 harmful cases plus 20 justified
controls across at least two held-out repositories, with both behavior classes
in development. This search frame cannot meet those denominators or provide a
balanced development set. Stop here: do not label, run RepoPilot, implement the
detector, or report accuracy. This is a no-go on the current pilot frame, not a
claim that the feature has no value.

The queries are text-ranked and the 10-result cap means this screen cannot say
how often the pattern occurs in the repositories or across GitHub. A wider
cohort would need a new pre-registered discovery method before any labels or
detector output are collected.
