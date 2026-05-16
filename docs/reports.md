# Reports

RepoPilot can render scan results as console, JSON, Markdown, HTML, and SARIF.

Use JSON when another tool, CI job, script, or future RepoPilot command needs to
read scan output.

```bash
repopilot scan . --format json --output repopilot-report.json
```

## JSON report schema

JSON scan reports include explicit schema metadata. The current schema is 0.10:

```json
{
  "schema_version": "0.10",
  "repopilot_version": "0.10.0",
  "root_path": ".",
  "files_count": 42,
  "directories_count": 12,
  "lines_of_code": 3200,
  "languages": [],
  "risk_summary": {
    "total": 0,
    "counts": { "p0": 0, "p1": 0, "p2": 0, "p3": 0 },
    "average_score": 0
  },
  "findings": []
}
```

### Top-level metadata

| Field | Type | Description |
|---|---|---|
| `schema_version` | string | RepoPilot JSON report schema version. |
| `repopilot_version` | string | RepoPilot binary version that produced the report. |
| `risk_summary` | object | Aggregate priority counts and average risk score derived from finding risk assessments. |

`schema_version` is intentionally separate from the binary version. Patch releases
may fix bugs without changing the report schema, while future minor releases can
evolve the schema in a documented way.

### Compatibility

RepoPilot keeps the existing scan summary fields at the top level of the JSON
document. Schema metadata and finding risk assessments are additive:

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
  "schema_version": "0.10",
  "repopilot_version": "0.10.0",
  "root_path": ".",
  "files_count": 42,
  "risk_summary": {
    "total": 12,
    "counts": { "p0": 0, "p1": 2, "p2": 5, "p3": 5 },
    "highest_priority": "P1",
    "average_score": 46
  },
  "baseline": {
    "path": ".repopilot/baseline.json",
    "new_findings": 2,
    "existing_findings": 10
  },
  "findings": []
}
```

## Audit receipt JSON

Use `--receipt` when a CI job, release process, or audit trail needs compact
evidence of what RepoPilot scanned without storing the full report:

```bash
repopilot scan . \
  --format markdown \
  --output repopilot-report.md \
  --receipt .repopilot/receipt.json
```

Receipt JSON is intentionally smaller than a scan report and has its own schema:

```json
{
  "schema_version": 1,
  "tool": "repopilot",
  "version": "0.10.0",
  "generated_at": "2026-05-16T00:00:00Z",
  "root_path": ".",
  "git": {
    "is_git_repo": true,
    "branch": "main",
    "commit": "abc123",
    "dirty": false
  },
  "scope": {
    "files_discovered": 42,
    "files_analyzed": 40,
    "directories_count": 12,
    "lines_of_code": 3200
  },
  "findings": {
    "total": 3,
    "critical": 0,
    "high": 1,
    "medium": 2,
    "low": 0,
    "info": 0
  },
  "languages": [],
  "health_score": 91
}
```

Receipts do not replace reports. Use reports for human review or downstream
finding details, and receipts for provenance, release evidence, and artifact
upload.

## Finding fields

Every finding includes stable fields documented in [rulesets.md](rulesets.md):

| Field | Type | Description |
|---|---|---|
| `id` | string | Stable finding ID derived from rule, path, and line. |
| `rule_id` | string | Stable rule identifier, for example `security.secret-candidate`. |
| `title` | string | Short human-readable summary. |
| `description` | string | Explanation of why the finding matters. |
| `recommendation` | string | Concrete remediation guidance. Older reports without this field still deserialize, but new reports include it. |
| `category` | string | Finding category. |
| `severity` | string | One of `INFO`, `LOW`, `MEDIUM`, `HIGH`, or `CRITICAL`. |
| `confidence` | string | One of `LOW`, `MEDIUM`, or `HIGH`; used to separate impact from certainty. |
| `risk` | object | Explainable prioritization assessment with `score`, `priority`, stable `signals`, and `formula_version`. |
| `docs_url` | string? | Optional documentation link for the rule. |
| `workspace_package` | string? | Optional monorepo package name. |
| `evidence` | array | One or more evidence locations. |

### Risk object

RepoPilot 0.10 uses `risk-v2` for deterministic, explainable prioritization:

```json
{
  "score": 67,
  "priority": "P2",
  "formula_version": "risk-v2",
  "signals": [
    {
      "id": "severity.medium",
      "label": "MEDIUM severity",
      "weight": 45,
      "reason": "base score from rule severity"
    },
    {
      "id": "cluster.repeated",
      "label": "repeated pattern",
      "weight": 7,
      "reason": "same rule appears repeatedly in the same repository area"
    }
  ]
}
```

Markdown and console reports include a Top Risk Clusters section that groups
repeated findings by rule and repository area. JSON keeps individual findings so
baselines, SARIF, and scripts can continue to address exact evidence locations.

See [Risk Engine](risk-engine.md) for priority buckets, signal families, and
calibration policy.

SARIF output carries the same category, recommendation, confidence, baseline
status, and workspace package metadata in result properties when available.

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
