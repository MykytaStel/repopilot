# Integrity corpus

Labeler: Claude (Anthropic, Opus 5.5) for the maintainer; same model wrote the detectors. Status: single-labeler, exploratory, pending maintainer review.
Every PR modifies or removes at least one test file.

## Held-out corpus

Denominator: merged, approved PRs (2026-03-01..05-31), sampled and labeled after tuning and evaluated once.

| Kind | agent | human |
|---|---|---|
| `skip-added` | 0/42 (0.0%, 95% CI 0.0–8.4%) | 1/56 (1.8%, 95% CI 0.3–9.4%) |
| `focus-added` | 0/42 (0.0%, 95% CI 0.0–8.4%) | 0/56 (0.0%, 95% CI 0.0–6.4%) |
| `test-removed` | 0/42 (0.0%, 95% CI 0.0–8.4%) | 1/56 (1.8%, 95% CI 0.3–9.4%) |
| `test-substituted` | 2/42 (4.8%, 95% CI 1.3–15.8%) | 5/56 (8.9%, 95% CI 3.9–19.3%) |
| `assertion-removed` | 1/42 (2.4%, 95% CI 0.4–12.3%) | 3/56 (5.4%, 95% CI 1.8–14.6%) |
| `assertion-trivialized` | 0/42 (0.0%, 95% CI 0.0–8.4%) | 1/56 (1.8%, 95% CI 0.3–9.4%) |
| `expectation-rewritten` | 4/42 (9.5%, 95% CI 3.8–22.1%) | 10/56 (17.9%, 95% CI 10.0–29.8%) |
| `suppression-added` | 4/42 (9.5%, 95% CI 3.8–22.1%) | 2/56 (3.6%, 95% CI 1.0–12.1%) |
| `gate-relaxed` | 0/42 (0.0%, 95% CI 0.0–8.4%) | 1/56 (1.8%, 95% CI 0.3–9.4%) |

| Verdict | agent | human |
|---|---|---|
| weakened | 0/42 (0.0%, 95% CI 0.0–8.4%) | 3/56 (5.4%, 95% CI 1.8–14.6%) |
| justified | 8/42 (19.0%, 95% CI 10.0–33.3%) | 17/56 (30.4%, 95% CI 19.9–43.3%) |
| none | 34/42 (81.0%, 95% CI 66.7–90.0%) | 36/56 (64.3%, 95% CI 51.2–75.5%) |

| Verdict after maintainer review | agent | human |
|---|---|---|
| weakened | 0/42 (0.0%, 95% CI 0.0–8.4%) | 2/56 (3.6%, 95% CI 1.0–12.1%) |
| justified | 8/42 (19.0%, 95% CI 10.0–33.3%) | 18/56 (32.1%, 95% CI 21.4–45.2%) |
| none | 34/42 (81.0%, 95% CI 66.7–90.0%) | 36/56 (64.3%, 95% CI 51.2–75.5%) |

Reviewed here: ih-070, ih-083, ih-084. Changed by the review:
- ih-070: weakened → justified; maintainer review 2026-10-02: justified, not weakened; lint still runs on Linux and Windows, and the macOS copy was a duplicate removed for CI speed

Unlabeled: 0 of 98.

### RepoPilot catches (blind labels)

PRs with both labels and results: 98. Binary: repopilot 0.25.0.
A PR counts once per signal kind, whatever the number of occurrences.

| Signal | caught | missed | false alarm | precision | recall |
|---|---|---|---|---|---|
| `integrity.test-focused` | 0 | 0 | 0 | n/a | n/a |
| `integrity.test-skipped` | 0 | 1 | 0 | n/a | 0/1 (0.0%, 95% CI 0.0–79.3%) |
| `integrity.test-removed` | 3 | 5 | 1 | 3/4 (75.0%, 95% CI 30.1–95.4%) | 3/8 (37.5%, 95% CI 13.7–69.4%) |
| `integrity.assertions-removed` | 3 | 2 | 1 | 3/4 (75.0%, 95% CI 30.1–95.4%) | 3/5 (60.0%, 95% CI 23.1–88.2%) |
| `integrity.suppression-added` | 5 | 1 | 1 | 5/6 (83.3%, 95% CI 43.6–97.0%) | 5/6 (83.3%, 95% CI 43.6–97.0%) |
| `integrity.gate-relaxed` | 1 | 0 | 0 | 1/1 (100.0%, 95% CI 20.7–100.0%) | 1/1 (100.0%, 95% CI 20.7–100.0%) |
| `integrity.test-removed+behavioral.test-deleted-or-emptied` | 5 | 3 | 1 | 5/6 (83.3%, 95% CI 43.6–97.0%) | 5/8 (62.5%, 95% CI 30.6–86.3%) |

### RepoPilot catches (labels with 1 reconciliation(s))

PRs with both labels and results: 98. Binary: repopilot 0.25.0.
A PR counts once per signal kind, whatever the number of occurrences.

| Signal | caught | missed | false alarm | precision | recall |
|---|---|---|---|---|---|
| `integrity.test-focused` | 0 | 0 | 0 | n/a | n/a |
| `integrity.test-skipped` | 0 | 1 | 0 | n/a | 0/1 (0.0%, 95% CI 0.0–79.3%) |
| `integrity.test-removed` | 3 | 5 | 1 | 3/4 (75.0%, 95% CI 30.1–95.4%) | 3/8 (37.5%, 95% CI 13.7–69.4%) |
| `integrity.assertions-removed` | 3 | 2 | 1 | 3/4 (75.0%, 95% CI 30.1–95.4%) | 3/5 (60.0%, 95% CI 23.1–88.2%) |
| `integrity.suppression-added` | 6 | 1 | 0 | 6/6 (100.0%, 95% CI 61.0–100.0%) | 6/7 (85.7%, 95% CI 48.7–97.4%) |
| `integrity.gate-relaxed` | 1 | 0 | 0 | 1/1 (100.0%, 95% CI 20.7–100.0%) | 1/1 (100.0%, 95% CI 20.7–100.0%) |
| `integrity.test-removed+behavioral.test-deleted-or-emptied` | 5 | 3 | 1 | 5/6 (83.3%, 95% CI 43.6–97.0%) | 5/8 (62.5%, 95% CI 30.6–86.3%) |

Reconciled after evaluation (evidence the blind label missed):
- ih-016: `# noqa: S105` (twice) and `# type: ignore` added in integration_tests/tests/data_seeder.py; in the labeling view (line 148) but past the first 90 lines the labeler read; found by re-reading every held-out and closed view in full after evaluation