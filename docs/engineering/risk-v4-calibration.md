# risk-v4 Calibration

Evidence for the `risk-v4` severity ceiling (v0.24 ledger RP24-005, RP24-006).
Generated with `scripts/risk_calibration.py` on 2026-09-25 from the same
corpus before (`risk-v3`) and after (`risk-v4`).

## Change

`risk-v4` keeps every `risk-v3` weight and score. The only change is a
severity ceiling on priority: context signals (baseline status, review diff,
graph, blast radius, clusters) still raise the score and reorder findings, but
a finding's priority cannot exceed its severity's ceiling.

| Severity | Highest priority |
|---|---|
| CRITICAL | P0 |
| HIGH | P0 |
| MEDIUM | P1 |
| LOW | P2 |
| INFO | P3 |

When the ceiling applies, the finding carries a zero-weight
`severity.priority-ceiling` signal that states the cap.

## Corpus

28 entries, one `repopilot scan` or `repopilot review` run each:

- every pinned zoo repository (11) in the `default` and `strict` profiles;
- RepoPilot's own tree in both profiles;
- strict `review` of four real changes: RepoPilot `1f17e2c7..6a49e4bb` (`self`)
  and the last five commits of three private application repositories
  (`local-a`, `local-b`, `local-c`; anonymized).

Counts are finding occurrences summed across all 28 entries.

## Result

| Severity | Before P0 | Before P1 | Before P2 | Before P3 | After P0 | After P1 | After P2 | After P3 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| CRITICAL | 2 | 0 | 0 | 0 | 2 | 0 | 0 | 0 |
| HIGH | 52 | 42 | 0 | 0 | 52 | 42 | 0 | 0 |
| MEDIUM | 4 | 276 | 1540 | 1 | 0 | 280 | 1540 | 1 |
| LOW | 0 | 0 | 306 | 1860 | 0 | 0 | 306 | 1860 |

Rules whose priority distribution changed:

| Corpus entry | Rule [severity] | Before | After |
|---|---|---|---|
| review strict local-b | `architecture.excessive-fan-out [MEDIUM]` | P0 2 | P1 2 |
| review strict local-c | `architecture.excessive-fan-out [MEDIUM]` | P0 1 | P1 1 |
| review strict local-c | `language.rust.panic-risk [MEDIUM]` | P0 1, P1 2 | P1 3 |

## Reading

- All four changes are MEDIUM findings that reached P0 only through review
  context (`baseline.new`, `review.in-diff`, graph hub, blast radius, and
  clusters) in changed-code reviews — the RP23-023 inflation.
- No CRITICAL, HIGH, or LOW finding changed priority, and no zoo or self-scan
  entry changed at all; `scan` output is unaffected in this corpus.
- `--fail-on-priority p0` no longer fails on those four findings;
  `--fail-on-priority p1` still does.

## Limits

The corpus is small and partly private. It shows the intended effect and the
absence of collateral changes on these repositories; it is not a claim about
every repository. Regenerate this table for any later formula change.
