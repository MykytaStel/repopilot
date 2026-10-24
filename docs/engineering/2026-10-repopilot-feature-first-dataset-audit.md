# RepoPilot feature-first and dataset audit

**Status:** audit and research artifacts plus an integrity-evaluation cache fix; detector behavior did not change.
**Snapshot:** 2026-10-24, `origin/main` / `05015bf5c84eaabe49e866b30e1761bfdb8a45dc`, release binary `repopilot 0.25.0`.

## Scope and result

This pass follows the approved feature-first direction: assess the Promise `.catch()` gap and test it against real changes before extending the detector. It also checks existing real-repository and PR corpora, then expands the discovery set without treating search results as labels.

The 11 pinned repositories in `tests/zoo` all matched their committed default and strict snapshots. The current run reported 25 default-visible findings: 9 labeled actionable, 16 valid-but-accepted, and 0 labeled false-positive. This is a reviewed expectation snapshot, not a measured precision estimate; the zoo is small and curated. The repeated hotspots are seven accepted secret-candidate findings in eShopOnWeb's intentional demo credentials and six accepted circular-dependency findings in Excalidraw.

The broader integrity corpus contains 358 PRs (198 development, 98 held-out, 62 closed agent PRs). It measures test-integrity changes only. Its labels are single-reviewer and exploratory; the same model wrote the detectors. The committed results are from `repopilot 0.23.0`. All 98 held-out cases were re-evaluated with `repopilot 0.25.0`; the binary hash, source commit, per-PR results, and report are saved in [the immutable reevaluation record](../../tests/integrity/evaluations/repopilot-0.25.0-holdout/metadata.toml). This rerun checks version drift on this corpus, not broad detector quality.

Using the existing reconciled labels, the per-PR counts changed as follows:

| Signal | 0.23.0 TP/FN/FP | 0.25.0 TP/FN/FP |
|---|---:|---:|
| `integrity.test-removed` | 3/5/2 | 3/5/1 |
| `integrity.assertions-removed` | 2/3/1 | 3/2/1 |
| `integrity.suppression-added` | 6/1/0 | 6/1/0 |
| `integrity.gate-relaxed` | 1/0/0 | 1/0/0 |
| `integrity.test-removed+behavioral.test-deleted-or-emptied` | 5/3/3 | 5/3/1 |

The test-removal combined signal counts a PR once across either detector. Positive denominators are small, labels remain exploratory, and changes are limited to this test-integrity sample.

The existing real-history benchmark contains six Python PRs with labels pending. The change-proof set contains eight controlled cases. Neither is a multi-language accuracy sample.

## Analytics follow-up — evaluation cache identity

The integrity-corpus evaluator previously cached results as `.integrity-corpus/results/<pr-id>.json`. Since it skipped existing files, evaluating the same PR with a different RepoPilot executable could silently reuse the old detector output. The result recorded a version string but the cache key did not identify the executable.

The evaluator now namespaces cache entries by the executable's SHA-256. Reports select a matching run with `report --bin`, refuse to guess when several builds are present, display the version and hash prefix for fingerprinted results, and preserve the committed baseline unless `--write-results` is passed. Old un-fingerprinted cache files are ignored by default because they may contain mixed builds; `--allow-legacy-cache` exposes them only for explicit historical inspection with a warning. Empty or missing selected runs produce an error instead of falling back to baseline results. Regression tests cover distinct outputs for two binaries, ambiguous selection, legacy opt-in, empty runs, and report identity.

The 405-test Python scripts suite passes. RepoPilot found no in-diff findings on the working-tree review; its verdict remains `REVIEW` because no verification checks were selected, and the review analyzed only 5 of 18 changed files. This makes corpus version comparisons reproducible; it does not increase detector accuracy or change detector behavior.

## Promise `.catch()` discovery

The initial frozen pilot contains 59 text-ranked PR search results from six popular JavaScript/TypeScript repositories. Diff review found four production PRs with added or changed `.catch()` handlers, across three repositories; only one resembles the target shape, and there are not enough cases for the predeclared held-out minimum of 20 harmful cases plus 20 justified controls. No labels were collected and no RepoPilot baseline was run.

A separate, broader discovery search found 197 PRs across 17 popular JavaScript/TypeScript repositories. Automated diff-hunk screening found 34 PRs with `.catch()` matches; 26 also have a match in a source-like path under a simple filename heuristic. That count includes context-only, unrelated, or semantically safe changes. Twelve patch retrievals failed in the connector. Ten selected patches were inspected manually; examples include handlers that log and preserve failure, report errors, handle optional dependencies, or intentionally suppress cleanup and best-effort errors. This is useful for finding controls and improving search, but it remains a text-ranked discovery frame with no labels, no held-out split, and no RepoPilot outputs. Spot checks illustrate the distinction: [Angular #69860](https://github.com/angular/angular/pull/69860) reports CDP failures to its caller; [Axios #11166](https://github.com/axios/axios/pull/11166) wraps and rejects the error; [TanStack Query #10771](https://github.com/TanStack/query/pull/10771) adds rejection reporting; and [Vite #23600](https://github.com/vitejs/vite/pull/23600) preserves an optional-peer fallback. These are possible controls, not adjudicated labels.

The expanded results are preserved in [the discovery manifest](../../tests/benchmarks/pilots/promise-catch-expanded/manifest.toml), [candidate pool](../../tests/benchmarks/pilots/promise-catch-expanded/candidate_pool.json), and [diff screening](../../tests/benchmarks/pilots/promise-catch-expanded/screening.json). They are separate from the original 59-PR pilot. PR code and tests were not executed.

**Decision:** do not implement Promise `.catch()` support from this evidence. The user's approved design makes implementation conditional on positive, independently labeled pilot evidence. These searches show there is more candidate material to screen, but do not establish harmful fallbacks or detector value.

## Other audit observations

- An earlier source review found no changed-scope findings or review signals. The MCP subtree scan showed zero visible findings; 31 targeted MCP tests passed. The final review record, including its coverage limits and explained cross-file move signals, is in [the quality expansion log](2026-10-repopilot-quality-expansion.md).
- The AI Context header in [header.rs](../../src/output/ai_context/header.rs) reports analyzed-file counts while [Repository Facts](../../src/output/ai_context/repo_facts.rs) reports a broader repository inventory. One captured run showed 823 analyzed files versus 1,900 total facts. The first quality slice added explicit scope labels and updated the golden outputs.
- Static `expect()` calls exist for the embedded Tree-sitter grammar at [parse.rs:78](../../src/analysis/parse.rs#L78) and for a progress token after a guard at [progress.rs:126](../../src/commands/mcp/progress.rs#L126). The review marked these as panic-risk candidates; neither was reproduced as a runtime bug.

## Dataset direction

Keep separate datasets for separate questions:

1. **Pinned repository snapshots (zoo):** default/strict output drift, reviewed expectation labels, and noisy-rule hotspots.
2. **Immutable PR changes:** changed-line recall, false alerts, and version comparisons. Freeze repository and time splits before annotation; never split the same repository across development and evaluation.
3. **Pattern pilots:** enriched candidate cases plus matched safe controls for a specific behavior such as Promise `.catch()`.
4. **Controlled fixtures:** precise supported-syntax and safe/unsafe regression guards. These establish mechanism contracts, not production accuracy.

For the next PR dataset, select repositories and a date window first, enumerate eligible merged PRs, then draw a deterministic sample from that frame. Pin base/head/merge SHAs and record selection provenance, language, changed paths, RepoPilot version/configuration, coverage, and reviewer decisions. Use two independent reviewers plus a recorded resolution for disagreements. Keep labels blind to RepoPilot output; freeze development labels before tuning and run the held-out set once. Report denominators, 95% intervals, analyzable coverage, evidence-location accuracy, and cost. Do not infer prevalence from a text-ranked query or call an unreviewed scan output “accuracy.”

LLM knowledge remains a later option. First establish licensed, provenance-tracked, independently labeled examples and stable held-out evaluation. Until then, use this data to improve tests and detector evaluation, not to train a model.

## Next actions

1. Manually screen the broader Promise `.catch()` candidate pool into harmful, justified, uncertain, and non-case rows; keep the discovery frame exploratory.
2. If enough cases remain, create a new preregistered, repository-disjoint development/evaluation pilot and apply the previously agreed thresholds. Implement only after it passes.
3. The AI Context scope labels are implemented and covered by command and golden tests.
4. Add cross-file handler moves as a review-signal quality case; evaluate any detector change on repository-disjoint moves and true removals before suppressing this class of signal.

The AI Context scope-label follow-up is implemented in
[`2026-10-repopilot-quality-expansion.md`](2026-10-repopilot-quality-expansion.md).

