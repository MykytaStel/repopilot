# Risk Model

RepoPilot uses `risk-v3`.

`risk-v3` is deterministic and local. It combines severity, confidence, category,
Knowledge Engine calibration, file role, baseline status, review diff context,
workspace hotspots, import graph impact, and repeated-rule clusters.

Scores stay on a 0-100 scale:

| Priority | Score | Meaning |
|---|---:|---|
| P0 | 90-100 | Immediate risk; fix or explicitly accept before release. |
| P1 | 70-89 | High-impact hardening; should be reviewed soon. |
| P2 | 40-69 | Maintainability, architecture, or moderate runtime risk. |
| P3 | 0-39 | Backlog cleanup or low-urgency review signal. |

Each risk signal has `id`, `label`, `weight`, `reason`, and `source`. Example:

```text
+75 severity: high severity finding
+12 security: security findings are prioritized
+8 graph: file is an import hub
-10 confidence: low confidence heuristic signal
```

See [Risk Engine](risk-engine.md) for the full scoring and calibration policy.
