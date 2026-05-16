# Risk Engine

RepoPilot risk scoring is local, deterministic, and explainable. It does not use
telemetry, hosted analysis, or machine learning. The goal is to rank review work
better than severity alone while keeping every score auditable.

## Formula v2

`risk-v2` scores each finding on a 0-100 scale:

```text
base impact
→ confidence adjustment
→ Knowledge Engine rule calibration
→ file and role context
→ baseline, review, workspace, graph, and cluster overlays
→ priority bucket
```

Priority buckets are stable:

| Priority | Score | Meaning |
|---|---:|---|
| P0 | 90-100 | Immediate risk; fix or explicitly accept before release. |
| P1 | 70-89 | High-impact hardening; should be reviewed soon. |
| P2 | 40-69 | Maintainability, architecture, or moderate runtime risk. |
| P3 | 0-39 | Backlog cleanup or low-urgency review signal. |

## Signals

Each finding includes `risk.signals[]` with a stable `id`, label, signed weight,
and reason. Common signal families:

- `severity.*` and `confidence.*` define the starting point.
- `knowledge.*` applies rule-specific calibration from the bundled Knowledge Engine.
- `role.*` adjusts for production, test, generated, config, domain, service, and controller files.
- `baseline.*` prioritizes new findings over accepted existing debt.
- `review.in-diff` and `review.blast-radius` prioritize changed lines and impacted import dependents.
- `workspace.hotspot` marks packages with repeated high-risk findings.
- `graph.*` marks dependency hubs and shared dependencies.
- `cluster.repeated` marks repeated rule patterns in the same repository area.

Signals are additive but clamped to the 0-100 score range. They are meant to
explain ranking, not prove that a finding is always a defect.

## Cluster-Aware Reports

Reports and AI plans group repeated findings by rule and repository area. This
keeps patterns such as many `unwrap()` calls in one renderer module from hiding
other risk clusters.

Raw JSON still includes each finding individually so downstream tools can keep
stable finding IDs, evidence, SARIF mapping, and baseline matching.

## Calibration Policy

Risk changes should include focused tests or fixtures that show the intended
ordering. RepoPilot keeps self-audit as a regression target: the repository
should not produce P0/P1 findings by default, and repeated medium findings should
surface as clusters rather than a long undifferentiated list.

## Limitations

RepoPilot uses static, heuristic signals. It is not a compiler, type checker, or
security proof. Treat findings as review evidence:

- confirm the cited code path before changing behavior;
- prefer focused suppressions or Knowledge Engine calibration over broad rule removal;
- add regression tests when a finding is a false positive in a supported context;
- use language-native tools such as `cargo clippy`, ESLint, Ruff, Pyright, `go vet`, and test suites alongside RepoPilot.
