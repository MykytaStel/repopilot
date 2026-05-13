# Reports

RepoPilot can render scan results as console, JSON, Markdown, HTML, and SARIF.

Use JSON when another tool, CI job, script, or future RepoPilot command needs to
read scan output.

```bash
repopilot scan . --format json --output repopilot-report.json
```

## JSON report schema

Starting with RepoPilot 0.9, JSON scan reports include explicit schema metadata:

```json
{
  "schema_version": "0.9",
  "repopilot_version": "0.9.0",
  "root_path": ".",
  "files_count": 42,
  "directories_count": 12,
  "lines_of_code": 3200,
  "languages": [],
  "findings": []
}
```

### Top-level metadata

| Field | Type | Description |
|---|---|---|
| `schema_version` | string | RepoPilot JSON report schema version. |
| `repopilot_version` | string | RepoPilot binary version that produced the report. |

`schema_version` is intentionally separate from the binary version. Patch releases
may fix bugs without changing the report schema, while future minor releases can
evolve the schema in a documented way.

### Compatibility

RepoPilot 0.9 keeps the existing scan summary fields at the top level of the JSON
document. The schema metadata is additive:

- existing consumers can continue reading fields such as `findings`,
  `files_count`, and `lines_of_code`;
- `repopilot compare` continues to read older JSON reports that do not contain
  `schema_version`;
- future tools can branch on `schema_version` when parsing reports.

## Baseline JSON reports

When a scan is rendered with a baseline, the JSON report also includes the same
schema metadata:

```bash
repopilot scan . \
  --baseline .repopilot/baseline.json \
  --format json \
  --output repopilot-baseline-report.json
```

Example shape:

```json
{
  "schema_version": "0.9",
  "repopilot_version": "0.9.0",
  "root_path": ".",
  "files_count": 42,
  "baseline": {
    "path": ".repopilot/baseline.json",
    "new_findings": 2,
    "existing_findings": 10
  },
  "findings": []
}
```

## Finding fields

Every finding includes stable fields documented in [rulesets.md](rulesets.md):

| Field | Type | Description |
|---|---|---|
| `id` | string | Stable finding ID derived from rule, path, and line. |
| `rule_id` | string | Stable rule identifier, for example `security.secret-candidate`. |
| `title` | string | Short human-readable summary. |
| `description` | string | Explanation of why the finding matters. |
| `category` | string | Finding category. |
| `severity` | string | One of `INFO`, `LOW`, `MEDIUM`, `HIGH`, or `CRITICAL`. |
| `docs_url` | string? | Optional documentation link for the rule. |
| `workspace_package` | string? | Optional monorepo package name. |
| `evidence` | array | One or more evidence locations. |

## Recommended usage

For local review:

```bash
repopilot scan . --format markdown --output repopilot-report.md
```

For scripts and comparisons:

```bash
repopilot scan . --format json --output before.json
repopilot scan . --format json --output after.json
repopilot compare before.json after.json --format markdown
```
