# Local Feedback

RepoPilot can read local suppressions from:

```text
.repopilot/feedback.yml
```

Minimal example:

```yaml
suppressions:
  - rule_id: architecture.large-file
    path: "src/generated/schema.rs"
    reason: generated schema boundary
  - kind: behavioral.network-call-added
    path: "src/generated/**"
    reason: generated client transport
    expires: "2026-12-31"
```

Run normally. RepoPilot applies matching suppressions before rendering console,
Markdown, JSON, review, and receipt output:

```bash
repopilot scan .
repopilot review .
```

Inspect local feedback before committing it:

```bash
repopilot inspect feedback .
repopilot inspect feedback . --format json
repopilot inspect feedback . --evaluate --format json
```

By default, `inspect feedback` only validates `.repopilot/feedback.yml` and
renders diagnostics. Use `--evaluate` when you need matched and unmatched
suppression results against current findings. Evaluation is heavier because it
runs a repository scan before applying local feedback.

Ignore local feedback when you want the raw report:

```bash
repopilot scan . --ignore-feedback
repopilot review . --ignore-feedback
```

RepoPilot validates feedback as structured YAML. Finding suppressions use
`rule_id + path`; review-signal suppressions use namespaced `kind + path`.
Include a reason for auditability. Review suppressions may include an ISO `expires` date;
expired entries no longer suppress or affect the gate.
Suppressions that do not match current findings are reported when feedback is
evaluated by `inspect feedback --evaluate`, `scan`, or `review`. Matching
suppressions are counted in `local_feedback`:

```json
{
  "local_feedback": {
    "feedback_path": ".repopilot/feedback.yml",
    "suppressions_loaded": 1,
    "review_suppressions_loaded": 1,
    "suppressed_findings_count": 1,
    "suppressed_review_signals_count": 1,
    "unmatched_suppressions_count": 0,
    "invalid_suppressions_count": 0,
    "unmatched_suppressions": [],
    "parse_error": null
  }
}
```

This is repository-local calibration, not remote learning. RepoPilot does not
upload feedback files, source code, or suppression history.

## Commit policy

Do not commit `.repopilot/feedback.yml` by default. Commit it only when the
suppressions are intentionally team-reviewed and part of repository policy.
Personal or temporary suppressions should stay uncommitted.
