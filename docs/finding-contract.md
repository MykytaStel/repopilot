# Finding Contract

RepoPilot 0.13.0 validates every rendered finding against an internal contract.
Validation runs locally after the single enrichment pass and risk scoring. Normal
scans report contract problems as diagnostics; tests and release checks should
fail on them.

A valid finding has:

- non-empty `id`, `rule_id`, `title`, `description`, and `recommendation`;
- at least one evidence item;
- non-empty evidence path and a `line_start` greater than zero;
- `line_end` greater than or equal to `line_start` when present;
- non-empty `risk.formula_version`;
- at least one risk signal;
- docs for high and critical findings.

The contract is intentionally strict because findings are used by baselines,
SARIF, JSON reports, AI context, and CI gates. RepoPilot should not silently
render incomplete findings or hide contract warnings through visibility filters.

The scan pipeline is:

```text
raw findings -> enrich once -> risk scoring -> contract validation -> report finalization
```

Enrichment fills missing title, description, recommendation, docs URL,
provenance, and the stable finding id before renderers see the finding. Contract
validation timing is exposed as `scan_timings.contract_validation_us`.

The validation model is exposed through `repopilot::api::findings` for embedded
tests and release validation.
