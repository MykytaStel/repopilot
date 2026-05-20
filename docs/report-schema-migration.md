# Report Schema Migration

RepoPilot report schemas are intentionally versioned because JSON reports are
useful in CI, dashboards, PR bots, and downstream tooling.

## Current direction

Schema `0.14` adds optional local feedback transparency:

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

RepoPilot `0.13.0` intentionally tightens current report readers. Commands such
as `repopilot compare` require current scan reports with both top-level
`schema_version` and `report.schema_version` matching the current schema.
Baseline files keep their separate baseline schema policy.
