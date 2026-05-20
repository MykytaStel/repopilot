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
```

Ignore local feedback when you want the raw report:

```bash
repopilot scan . --ignore-feedback
repopilot review . --ignore-feedback
```

RepoPilot validates feedback as structured YAML. Malformed YAML, entries without
`rule_id` or `path`, and suppressions that do not match current findings are
reported as diagnostics. Matching suppressions are counted in `local_feedback`:

```json
{
  "local_feedback": {
    "feedback_path": ".repopilot/feedback.yml",
    "suppressions_loaded": 1,
    "suppressed_findings_count": 1,
    "unmatched_suppressions_count": 0,
    "invalid_suppressions_count": 0,
    "unmatched_suppressions": [],
    "parse_error": null
  }
}
```

This is repository-local calibration, not remote learning. RepoPilot does not
upload feedback files, source code, or suppression history.
