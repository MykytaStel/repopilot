# Integrity corpus

Labeler: Claude (Anthropic, Opus 5.5) for the maintainer; same model wrote the detectors. Status: single-labeler, exploratory, pending maintainer review.
Every PR modifies or removes at least one test file.

## Development corpus

Denominator: merged, approved PRs (2026-06-01..09-28); detectors were tuned on it after the first evaluation.

| Kind | agent | human |
|---|---|---|
| `skip-added` | 0/84 (0.0%, 95% CI 0.0–4.4%) | 0/114 (0.0%, 95% CI 0.0–3.3%) |
| `focus-added` | 0/84 (0.0%, 95% CI 0.0–4.4%) | 0/114 (0.0%, 95% CI 0.0–3.3%) |
| `test-removed` | 3/84 (3.6%, 95% CI 1.2–10.0%) | 5/114 (4.4%, 95% CI 1.9–9.9%) |
| `test-substituted` | 2/84 (2.4%, 95% CI 0.7–8.3%) | 3/114 (2.6%, 95% CI 0.9–7.5%) |
| `assertion-removed` | 0/84 (0.0%, 95% CI 0.0–4.4%) | 3/114 (2.6%, 95% CI 0.9–7.5%) |
| `assertion-trivialized` | 0/84 (0.0%, 95% CI 0.0–4.4%) | 0/114 (0.0%, 95% CI 0.0–3.3%) |
| `expectation-rewritten` | 16/84 (19.0%, 95% CI 12.1–28.7%) | 31/114 (27.2%, 95% CI 19.9–36.0%) |
| `suppression-added` | 1/84 (1.2%, 95% CI 0.2–6.4%) | 8/114 (7.0%, 95% CI 3.6–13.2%) |
| `gate-relaxed` | 2/84 (2.4%, 95% CI 0.7–8.3%) | 1/114 (0.9%, 95% CI 0.2–4.8%) |

| Verdict | agent | human |
|---|---|---|
| weakened | 1/84 (1.2%, 95% CI 0.2–6.4%) | 0/114 (0.0%, 95% CI 0.0–3.3%) |
| justified | 21/84 (25.0%, 95% CI 17.0–35.2%) | 45/114 (39.5%, 95% CI 31.0–48.6%) |
| none | 62/84 (73.8%, 95% CI 63.5–82.0%) | 69/114 (60.5%, 95% CI 51.4–69.0%) |

| Verdict after maintainer review | agent | human |
|---|---|---|
| weakened | 0/84 (0.0%, 95% CI 0.0–4.4%) | 0/114 (0.0%, 95% CI 0.0–3.3%) |
| justified | 22/84 (26.2%, 95% CI 18.0–36.5%) | 45/114 (39.5%, 95% CI 31.0–48.6%) |
| none | 62/84 (73.8%, 95% CI 63.5–82.0%) | 69/114 (60.5%, 95% CI 51.4–69.0%) |

Reviewed here: ic-069. Changed by the review:
- ic-069: weakened → justified; maintainer review 2026-10-02: justified, not weakened; the new test follows the file's existing convention (10 identical eslint-disable-next-line comments before `{} as any`)

Unlabeled: 0 of 198.

### RepoPilot catches (blind labels)

PRs with both labels and results: 198. Binary: repopilot 0.23.0.
A PR counts once per signal kind, whatever the number of occurrences.

| Signal | caught | missed | false alarm | precision | recall |
|---|---|---|---|---|---|
| `integrity.test-focused` | 0 | 0 | 0 | n/a | n/a |
| `integrity.test-skipped` | 0 | 0 | 0 | n/a | n/a |
| `integrity.test-removed` | 10 | 3 | 13 | 10/23 (43.5%, 95% CI 25.6–63.2%) | 10/13 (76.9%, 95% CI 49.7–91.8%) |
| `integrity.assertions-removed` | 1 | 2 | 4 | 1/5 (20.0%, 95% CI 3.6–62.4%) | 1/3 (33.3%, 95% CI 6.1–79.2%) |
| `integrity.suppression-added` | 9 | 0 | 4 | 9/13 (69.2%, 95% CI 42.4–87.3%) | 9/9 (100.0%, 95% CI 70.1–100.0%) |
| `integrity.gate-relaxed` | 1 | 2 | 0 | 1/1 (100.0%, 95% CI 20.7–100.0%) | 1/3 (33.3%, 95% CI 6.1–79.2%) |
| `integrity.test-removed+behavioral.test-deleted-or-emptied` | 13 | 0 | 13 | 13/26 (50.0%, 95% CI 32.1–67.9%) | 13/13 (100.0%, 95% CI 77.2–100.0%) |

### RepoPilot catches (labels with 4 reconciliation(s))

PRs with both labels and results: 198. Binary: repopilot 0.23.0.
A PR counts once per signal kind, whatever the number of occurrences.

| Signal | caught | missed | false alarm | precision | recall |
|---|---|---|---|---|---|
| `integrity.test-focused` | 0 | 0 | 0 | n/a | n/a |
| `integrity.test-skipped` | 0 | 0 | 0 | n/a | n/a |
| `integrity.test-removed` | 10 | 3 | 13 | 10/23 (43.5%, 95% CI 25.6–63.2%) | 10/13 (76.9%, 95% CI 49.7–91.8%) |
| `integrity.assertions-removed` | 1 | 2 | 4 | 1/5 (20.0%, 95% CI 3.6–62.4%) | 1/3 (33.3%, 95% CI 6.1–79.2%) |
| `integrity.suppression-added` | 13 | 0 | 0 | 13/13 (100.0%, 95% CI 77.2–100.0%) | 13/13 (100.0%, 95% CI 77.2–100.0%) |
| `integrity.gate-relaxed` | 1 | 2 | 0 | 1/1 (100.0%, 95% CI 20.7–100.0%) | 1/3 (33.3%, 95% CI 6.1–79.2%) |
| `integrity.test-removed+behavioral.test-deleted-or-emptied` | 13 | 0 | 13 | 13/26 (50.0%, 95% CI 32.1–67.9%) | 13/13 (100.0%, 95% CI 77.2–100.0%) |

Reconciled after evaluation (evidence the blind label missed):
- ic-030: `# noqa: PLW0717` added in src/devopness/core/api_error.py, a source file outside the labeling view; confirmed in the PR patch
- ic-121: `/* eslint-disable */` added in a generated API source file outside the labeling view; confirmed in the PR patch
- ic-142: `# noqa: BLE001` added in jumpstarter/client/lease.py, a source file outside the labeling view; confirmed in the PR patch
- ic-177: `// eslint-disable-next-line no-await-in-loop` added in src/checkDepsStatus.ts, outside the labeling view; confirmed in the PR patch

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

PRs with both labels and results: 98. Binary: repopilot 0.23.0.
A PR counts once per signal kind, whatever the number of occurrences.

| Signal | caught | missed | false alarm | precision | recall |
|---|---|---|---|---|---|
| `integrity.test-focused` | 0 | 0 | 0 | n/a | n/a |
| `integrity.test-skipped` | 0 | 1 | 0 | n/a | 0/1 (0.0%, 95% CI 0.0–79.3%) |
| `integrity.test-removed` | 3 | 5 | 2 | 3/5 (60.0%, 95% CI 23.1–88.2%) | 3/8 (37.5%, 95% CI 13.7–69.4%) |
| `integrity.assertions-removed` | 2 | 3 | 1 | 2/3 (66.7%, 95% CI 20.8–93.9%) | 2/5 (40.0%, 95% CI 11.8–76.9%) |
| `integrity.suppression-added` | 5 | 1 | 1 | 5/6 (83.3%, 95% CI 43.6–97.0%) | 5/6 (83.3%, 95% CI 43.6–97.0%) |
| `integrity.gate-relaxed` | 1 | 0 | 0 | 1/1 (100.0%, 95% CI 20.7–100.0%) | 1/1 (100.0%, 95% CI 20.7–100.0%) |
| `integrity.test-removed+behavioral.test-deleted-or-emptied` | 5 | 3 | 3 | 5/8 (62.5%, 95% CI 30.6–86.3%) | 5/8 (62.5%, 95% CI 30.6–86.3%) |

### RepoPilot catches (labels with 1 reconciliation(s))

PRs with both labels and results: 98. Binary: repopilot 0.23.0.
A PR counts once per signal kind, whatever the number of occurrences.

| Signal | caught | missed | false alarm | precision | recall |
|---|---|---|---|---|---|
| `integrity.test-focused` | 0 | 0 | 0 | n/a | n/a |
| `integrity.test-skipped` | 0 | 1 | 0 | n/a | 0/1 (0.0%, 95% CI 0.0–79.3%) |
| `integrity.test-removed` | 3 | 5 | 2 | 3/5 (60.0%, 95% CI 23.1–88.2%) | 3/8 (37.5%, 95% CI 13.7–69.4%) |
| `integrity.assertions-removed` | 2 | 3 | 1 | 2/3 (66.7%, 95% CI 20.8–93.9%) | 2/5 (40.0%, 95% CI 11.8–76.9%) |
| `integrity.suppression-added` | 6 | 1 | 0 | 6/6 (100.0%, 95% CI 61.0–100.0%) | 6/7 (85.7%, 95% CI 48.7–97.4%) |
| `integrity.gate-relaxed` | 1 | 0 | 0 | 1/1 (100.0%, 95% CI 20.7–100.0%) | 1/1 (100.0%, 95% CI 20.7–100.0%) |
| `integrity.test-removed+behavioral.test-deleted-or-emptied` | 5 | 3 | 3 | 5/8 (62.5%, 95% CI 30.6–86.3%) | 5/8 (62.5%, 95% CI 30.6–86.3%) |

Reconciled after evaluation (evidence the blind label missed):
- ih-016: `# noqa: S105` (twice) and `# type: ignore` added in integration_tests/tests/data_seeder.py; in the labeling view (line 148) but past the first 90 lines the labeler read; found by re-reading every held-out and closed view in full after evaluation

## Closed agent PRs

Denominator: agent PRs closed without merge after discussion (2026-06-01..09-28).

| Kind | agent-closed |
|---|---|
| `skip-added` | 0/62 (0.0%, 95% CI 0.0–5.8%) |
| `focus-added` | 0/62 (0.0%, 95% CI 0.0–5.8%) |
| `test-removed` | 3/62 (4.8%, 95% CI 1.7–13.3%) |
| `test-substituted` | 7/62 (11.3%, 95% CI 5.6–21.5%) |
| `assertion-removed` | 1/62 (1.6%, 95% CI 0.3–8.6%) |
| `assertion-trivialized` | 1/62 (1.6%, 95% CI 0.3–8.6%) |
| `expectation-rewritten` | 8/62 (12.9%, 95% CI 6.7–23.4%) |
| `suppression-added` | 2/62 (3.2%, 95% CI 0.9–11.0%) |
| `gate-relaxed` | 0/62 (0.0%, 95% CI 0.0–5.8%) |

| Verdict | agent-closed |
|---|---|
| weakened | 2/62 (3.2%, 95% CI 0.9–11.0%) |
| justified | 16/62 (25.8%, 95% CI 16.6–37.9%) |
| none | 44/62 (71.0%, 95% CI 58.7–80.8%) |

| Verdict after maintainer review | agent-closed |
|---|---|
| weakened | 1/62 (1.6%, 95% CI 0.3–8.6%) |
| justified | 17/62 (27.4%, 95% CI 17.9–39.6%) |
| none | 44/62 (71.0%, 95% CI 58.7–80.8%) |

Reviewed here: icc-018, icc-059. Changed by the review:
- icc-059: weakened → justified; maintainer review 2026-10-02: justified, not weakened; the PR description explains the redesign (the operator renders the Alertmanager config, and alerts are suppressed through AlertExceptions)

Unlabeled: 0 of 62.

### RepoPilot catches (blind labels)

PRs with both labels and results: 62. Binary: repopilot 0.23.0.
A PR counts once per signal kind, whatever the number of occurrences.

| Signal | caught | missed | false alarm | precision | recall |
|---|---|---|---|---|---|
| `integrity.test-focused` | 0 | 0 | 0 | n/a | n/a |
| `integrity.test-skipped` | 0 | 0 | 1 | 0/1 (0.0%, 95% CI 0.0–79.3%) | n/a |
| `integrity.test-removed` | 5 | 4 | 0 | 5/5 (100.0%, 95% CI 56.6–100.0%) | 5/9 (55.6%, 95% CI 26.7–81.1%) |
| `integrity.assertions-removed` | 0 | 2 | 0 | n/a | 0/2 (0.0%, 95% CI 0.0–65.8%) |
| `integrity.suppression-added` | 2 | 0 | 0 | 2/2 (100.0%, 95% CI 34.2–100.0%) | 2/2 (100.0%, 95% CI 34.2–100.0%) |
| `integrity.gate-relaxed` | 0 | 0 | 0 | n/a | n/a |
| `integrity.test-removed+behavioral.test-deleted-or-emptied` | 7 | 2 | 0 | 7/7 (100.0%, 95% CI 64.6–100.0%) | 7/9 (77.8%, 95% CI 45.3–93.7%) |
