# Local Feedback

RepoPilot can read local suppressions from:

```text
.repopilot/feedback.yml
```

Minimal example:

```yaml
suppressions:
  - rule_id: architecture.large-file
    path: src/generated/schema.rs
    reason: generated schema boundary
```

Run normally:

```bash
repopilot scan .
```

Ignore local feedback when you want the raw report:

```bash
repopilot scan . --ignore-feedback
```

This is repository-local calibration, not remote learning.
