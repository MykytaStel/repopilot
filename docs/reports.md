# Reports

RepoPilot can render scan results as console, JSON, Markdown, HTML, and SARIF.

Use JSON when another tool, CI job, script, or future RepoPilot command needs to
read scan output.

```bash
repopilot scan . --format json --output repopilot-report.json
```

## JSON report schema

JSON scan reports include explicit schema metadata. The current schema is 0.18:

```json
{
  "schema_version": "0.18",
  "repopilot_version": "0.16.0",
  "report": {
    "kind": "scan",
    "schema_version": "0.18",
    "repopilot_version": "0.16.0"
  },
  "root_path": ".",
  "files_analyzed": 42,
  "directories_count": 12,
  "non_empty_lines": 3200,
  "languages": [],
  "risk_summary": {
    "total": 3,
    "counts": { "p0": 0, "p1": 1, "p2": 2, "p3": 0 },
    "average_score": 58
  },
  "raw_findings_count": 12,
  "visible_findings_count": 3,
  "hidden_suggestions_count": 9,
  "raw_signal_quality": {
    "findings_total": 12,
    "evidence_coverage_percent": 100,
    "recommendation_coverage_percent": 100,
    "docs_coverage_for_high_risk_percent": 100,
    "contract_violations": 0
  },
  "visible_signal_quality": {
    "findings_total": 3,
    "evidence_coverage_percent": 100,
    "recommendation_coverage_percent": 100,
    "docs_coverage_for_high_risk_percent": 100,
    "contract_violations": 0
  },
  "signal_quality": {
    "findings_total": 3,
    "evidence_coverage_percent": 100,
    "recommendation_coverage_percent": 100,
    "docs_coverage_for_high_risk_percent": 100,
    "contract_violations": 0
  },
  "local_feedback": {
    "feedback_path": ".repopilot/feedback.yml",
    "suppressions_loaded": 1,
    "suppressed_findings_count": 1,
    "unmatched_suppressions_count": 0,
    "invalid_suppressions_count": 0,
    "unmatched_suppressions": [],
    "parse_error": null
  },
  "diagnostics": [],
  "findings": []
}
```

### Top-level metadata

| Field | Type | Description |
|---|---|---|
| `schema_version` | string | RepoPilot JSON report schema version. |
| `repopilot_version` | string | RepoPilot binary version that produced the report. |
| `report` | object | Versioned report envelope for consumers that prefer metadata under one stable object. |
| `risk_summary` | object | Aggregate priority counts and average risk score derived from finding risk assessments. |
| `raw_findings_count` | number | Findings before the default visibility profile hides strict-only suggestions. |
| `visible_findings_count` | number | Findings rendered in the current report after visibility, feedback, and explicit filters. |
| `hidden_suggestions_count` | number | Findings hidden by the default profile but available through `--profile strict`. |
| `raw_signal_quality` | object | Aggregate confidence, lifecycle, source, coverage, and contract metrics before visibility filtering. |
| `visible_signal_quality` | object | Aggregate quality for findings visible in this report. |
| `signal_quality` | object | Compatibility alias for `visible_signal_quality`. |
| `cache_telemetry` | object | Optional changed-scan cache summary with hits, misses, changed-file reasons, per-file cache decisions, and cache timing impact. |
| `scan_timings` | object | Optional engine timing metadata. `file_scan_us` remains the compatibility aggregate; newer fields break out `discovery_us`, `file_analysis_us`, `parse_us` (tree-sitter parsing within file analysis), `enrichment_us`, `risk_scoring_us`, `contract_validation_us`, and `report_finalization_us`. |
| `local_feedback` | object | Optional summary of `.repopilot/feedback.yml` suppressions applied during this scan or review. |
| `diagnostics` | array | Optional structured warnings/errors captured during a scan, such as workspace partial failures. |

Diagnostics use `{ code, severity, message, path? }`. Recoverable diagnostics
with `warning` severity are report-only and keep the scan exit code at `0`
unless a finding gate fails. A reportable `error` diagnostic is written into the
requested report/receipt and then exits with RepoPilot runtime code `3`.

`schema_version` is intentionally separate from the binary version. Patch releases
may fix bugs without changing the report schema, while future minor releases can
evolve the schema in a documented way.

Binary `0.17.x` continues to emit schema `0.18` unless the serialized contract
changes. Schema numbers are monotonic contract revisions, not predictions of the
next RepoPilot package version.

### Compatibility

RepoPilot renders JSON through explicit report DTOs instead of serializing the
internal scan model directly. Schema `0.13` intentionally renamed scope counters
for accuracy:

- consumers should read `files_analyzed`, `non_empty_lines`, and
  `large_files_skipped`;
- future tools can branch on `schema_version` when parsing reports;
- consumers should prefer `report.schema_version` when they want a single
  envelope object, while top-level `schema_version` remains present for direct
  scripts.

Schema `0.14` adds optional `local_feedback` metadata to scan, baseline-scan,
review, and receipt output. `repopilot compare`
requires current scan reports with `schema_version` and `report.schema_version`
matching the current schema.

Schema `0.15` adds finding provenance, typed risk signal sources, `risk-v3`,
finding-contract diagnostics, and `signal_quality` metrics.

Schema `0.16` adds context graph report and cache diagnostics. Schema `0.17`
adds raw-vs-visible finding and signal-quality metrics so default-profile
reports do not look clean when meaningful strict-only findings were hidden.
Schema `0.18` adds the stable review-signal contract, suppression/gate metadata,
and the explicit review gate result.

Migration from pre-`0.13` reports is intentionally consumer-owned:

| Historical field | Current field |
|---|---|
| `files_count` | `files_analyzed` |
| `lines_of_code` | `non_empty_lines` |
| `skipped_files_count` | `large_files_skipped` |

The current reader accepts `0.16`, `0.17`, and `0.18` scan reports during the
transition. `repopilot compare` remains stricter: both top-level and envelope
schema versions must match a supported current report shape. Baseline files
follow their separate baseline schema policy.

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
  "schema_version": "0.18",
  "repopilot_version": "0.16.0",
  "report": {
    "kind": "baseline-scan",
    "schema_version": "0.18",
    "repopilot_version": "0.16.0"
  },
  "root_path": ".",
  "files_analyzed": 42,
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
  "diagnostics": [],
  "findings": []
}
```

## Review JSON reports

`repopilot review --format json` uses the same envelope policy with
`report.kind = "review"`. Review reports include scan scope, changed files,
blast-radius files, `risk_summary`, structured diagnostics, baseline metadata,
optional `local_feedback`, CI gate metadata when requested, and per-finding
`in_diff` / `baseline_status` classification. `tiered_signals` entries include
`signal_id`, namespaced `kind`, `family`, `tier`, `confidence`, path and line
ranges, merged evidence lines, headline/detail/blast radius, provenance,
suppression state, and gate eligibility. `review_gate` is independent from the
finding-only `ci_gate`. `review_timings` reports `diff_loading_us`,
`review_signals_us`, `gating_us`, and `rendering_us`.

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
  "schema_version": 5,
  "report": {
    "kind": "receipt",
    "schema_version": "5",
    "repopilot_version": "0.16.0"
  },
  "tool": "repopilot",
  "version": "0.16.0",
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
    "non_empty_lines": 3200
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
  "diagnostics": [],
  "local_feedback": null,
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
| `provenance` | object | Detector, rule lifecycle, signal source, and analysis scope for explaining where the finding came from. |
| `docs_url` | string? | Optional documentation link for the rule. |
| `workspace_package` | string? | Optional monorepo package name. |
| `evidence` | array | One or more evidence locations. |

### Risk object

RepoPilot uses `risk-v3` for deterministic, explainable prioritization:

```json
{
  "score": 67,
  "priority": "P2",
  "formula_version": "risk-v3",
  "signals": [
    {
      "id": "severity.medium",
      "label": "severity",
      "weight": 45,
      "reason": "medium severity finding",
      "source": "severity"
    },
    {
      "id": "cluster.repeated",
      "label": "cluster",
      "weight": 7,
      "reason": "same rule appears repeatedly in the same repository area",
      "source": "cluster"
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


## Machine-readable DTO ownership

All machine-readable JSON DTOs live in `report::schema`:

- scan JSON reports;
- baseline scan JSON reports;
- review JSON reports;
- report envelope parsing helpers.

Output modules should render these DTOs instead of owning separate schema
structures. This keeps CLI output, embedded API usage, and compatibility parsing
aligned.
