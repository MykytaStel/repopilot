# Report Schema Migration

RepoPilot report schemas are intentionally versioned because JSON reports are
useful in CI, dashboards, PR bots, and downstream tooling.

## Current direction

Schema `0.18` is the current report schema. It keeps `0.17` scan accounting and
adds stable review-signal identity, provenance, suppression, and gate metadata.

| Field | Where | Why |
|---|---|---|
| `raw_findings_count` | scan, baseline-scan | Shows how many findings existed before default visibility filtering. |
| `visible_findings_count` | scan, baseline-scan | Shows the findings rendered in the current report. |
| `hidden_suggestions_count` | scan, baseline-scan | Shows strict-only suggestions hidden from the default report. |
| `raw_signal_quality` | scan, baseline-scan, review | Summarizes quality before visibility filtering. |
| `visible_signal_quality` | scan, baseline-scan, review | Summarizes quality for rendered findings. |
| `signal_quality` | scan, baseline-scan, review | Compatibility alias for `visible_signal_quality`. |
| `provenance` | finding | Explains detector, lifecycle, signal source, and analysis scope. |
| `risk.signals[].source` | finding risk signal | Gives machine-readable source families for risk explanations. |
| `scan_timings.contract_validation_us` | scan timings | Exposes finding-contract validation timing separately. |
| `tiered_signals[].signal_id` | review signal | Stable identity derived from kind and path. |
| `tiered_signals[].kind` | review signal | Namespaced signal kind such as `taint.sql`. |
| `tiered_signals[].provenance` | review signal | Detector, lifecycle, source, and analysis scope. |
| `tiered_signals[].suppressed` | review signal | Shows local policy suppression without deleting JSON evidence. |
| `tiered_signals[].gate_eligible` | review signal | Separates advisory/noise signals from explicit policy gates. |
| `review_gate` | review | Result of the independent `--fail-on-review` policy. |

RepoPilot's current reader accepts `0.16`, `0.17`, and `0.18` scan reports
during the transition. New reports are emitted as `0.18`.

Schema `0.14` added optional local feedback transparency:

| Field | Where | Why |
|---|---|---|
| `local_feedback` | scan, baseline-scan, review, receipt | Shows how many `.repopilot/feedback.yml` suppressions were loaded, matched, unmatched, invalid, or blocked by parse errors. |

The field is omitted when no local feedback file was applied or when the command
uses `--ignore-feedback`.

## 0.13 accounting migration

Schema `0.13` clarifies scan accounting names:

| Old field | New field | Why |
|---|---|---|
| `files_count` | `files_analyzed` | The value counts analyzed text files, not all discovered files. |
| `lines_of_code` | `non_empty_lines` | RepoPilot counts non-empty lines used by audit thresholds. |
| `skipped_files_count` | `large_files_skipped` | The field describes large files skipped by size/limit rules. |

## Consumer guidance

Consumers should prefer:

```json
{
  "files_analyzed": 42,
  "non_empty_lines": 3200,
  "large_files_skipped": 1
}
```

If a downstream consumer still needs historical reports, keep that compatibility
in the consumer instead of relying on RepoPilot's current reader:

```text
files_analyzed <- files_count
non_empty_lines <- lines_of_code
large_files_skipped <- skipped_files_count
```

## Compatibility policy

RepoPilot intentionally tightens current report readers. Commands such as
`repopilot compare` require current scan reports with both top-level
`schema_version` and `report.schema_version` matching the current schema.
Baseline files keep their separate baseline schema policy.
