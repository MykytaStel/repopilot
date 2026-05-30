# Performance Budgets

RepoPilot scan performance is part of the product contract. A local-first safety
pass must stay fast enough for pre-commit, AI-agent loops, and CI adoption.

## Initial Budgets

These budgets start with RepoPilot self-scan fixtures and should be tightened as
larger fixture repositories are added:

| Scenario | Budget |
|---|---:|
| Warm full scan accounted engine time | <= 600ms |
| Report finalization | <= 100ms |
| Changed scan cache lookup and reuse | No material regression after cache warmup |

A benchmark update should be explicit when a legitimate scan-engine improvement
changes expected work. Otherwise, avoid merging changes that regress an existing
budget by more than 10%.

## Measurement

Use:

```bash
repopilot scan . --profile strict --timing --format json --output /tmp/repopilot-self-scan.json
```

Track `scan_timings.accounted_engine_us()` from the timing breakdown and
`scan_timings.report_finalization_us` from the JSON report. Keep wall-clock
numbers as supporting context only; the budget should focus on accounted engine
timing so CI host variance does not create noisy failures.

## Benchmark

A Criterion benchmark builds a synthetic 480-file multi-language repository
(Rust, TypeScript, Python, Go) in a temp directory and measures full-scan wall
time:

```bash
cargo bench --bench scan_bench
```

The benchmark lives in `benches/scan_bench.rs`. Use it to record a baseline
before scan-engine changes — notably the shared parsed-AST work — and to prove
the expected speedup afterwards. Criterion stores results under
`target/criterion` and reports the delta versus the previous run, so benchmark
`main` first, then the change branch.

## Fixture Direction

Add small, medium, and large synthetic repositories before enforcing this as a
hard CI gate. The gate should fail on major regressions only after fixture timing
is stable on the project CI runners. The `scan_bench` synthetic repository is the
first such fixture.
