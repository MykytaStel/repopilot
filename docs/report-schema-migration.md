# Report Schema Migration

RepoPilot report schemas are intentionally versioned because JSON reports are
useful in CI, dashboards, PR bots, and downstream tooling.

## Current direction

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

For older reports, consumers can fall back to:

```text
files_analyzed <- files_count
non_empty_lines <- lines_of_code
large_files_skipped <- skipped_files_count
```

## Compatibility policy

RepoPilot should keep reading older reports where practical, especially for
`repopilot compare`, baseline workflows, and older CI artifacts.
